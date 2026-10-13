#!/usr/bin/env python3
"""Verify HyperAI TUI resize, Ctrl+C, and test-only panic cleanup in a POSIX PTY."""

from __future__ import annotations

import argparse
import http.client
import fcntl
import hashlib
import json
import os
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
import unicodedata
from collections.abc import Callable
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit


def contains_visible_sha256_after(text: str, label: str) -> bool:
    compact_text = re.sub(r"\s+", "", text)
    compact_label = re.sub(r"\s+", "", label)
    pattern = rf"{re.escape(compact_label)}[0-9a-f]{{64}}(?![0-9a-f])"
    return re.search(pattern, compact_text) is not None


def run_cli(binary: Path, *arguments: str | Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(binary), *(str(argument) for argument in arguments)],
        capture_output=True,
        check=False,
        text=True,
        timeout=20,
    )


def require_success(result: subprocess.CompletedProcess[str], operation: str) -> None:
    if result.returncode != 0:
        raise RuntimeError(f"{operation} failed with exit status {result.returncode}")


def seed_market_history(binary: Path, home: Path) -> dict[str, str]:
    comparison_result = run_cli(binary, "market", "compare", "--home", home)
    require_success(comparison_result, "synthetic market comparison")
    comparison = json.loads(comparison_result.stdout)
    comparison_digest = comparison["digest"]

    preview_result = run_cli(
        binary,
        "market",
        "quote-preview",
        "--comparison-digest",
        comparison_digest,
        "--home",
        home,
    )
    require_success(preview_result, "synthetic quote preview")
    preview = json.loads(preview_result.stdout)
    preview_digest = preview["body"]["quote_preview"]["digest"]

    approval_result = run_cli(
        binary,
        "market",
        "quote-approve",
        "--preview-digest",
        preview_digest,
        "--home",
        home,
    )
    require_success(approval_result, "synthetic quote approval")
    approval = json.loads(approval_result.stdout)
    deal_digest = approval["deal_digest"]

    completion_result = run_cli(
        binary,
        "market",
        "quote-complete",
        "--preview-digest",
        preview_digest,
        "--deal-digest",
        deal_digest,
        "--home",
        home,
    )
    require_success(completion_result, "synthetic mock completion")

    history_result = run_cli(
        binary, "market", "history", "--limit", "20", "--home", home
    )
    require_success(history_result, "authenticated synthetic market history")
    history = json.loads(history_result.stdout)
    entries = history["body"]["entries"]
    if len(entries) != 1:
        raise RuntimeError("temporary service did not retain exactly one synthetic deal")
    entry = entries[0]
    if entry["approval"]["deal_digest"] != deal_digest:
        raise RuntimeError("temporary service returned a different synthetic deal")
    if entry["completion_receipt"] is None:
        raise RuntimeError("temporary service did not retain the mock receipt")
    return {"deal_digest": deal_digest, "preview_digest": preview_digest}


def authenticated_local_post(home: Path, route: str) -> dict[str, object]:
    runtime = home / "runtime"
    record = json.loads((runtime / "service.json").read_text(encoding="utf-8"))
    address = record["address"]
    parsed = urlsplit(f"http://{address}")
    if parsed.hostname != "127.0.0.1" or parsed.port is None:
        raise RuntimeError("synthetic setup service was not bound to IPv4 loopback")
    bearer = (runtime / "bearer.token").read_text(encoding="utf-8").strip()
    connection = http.client.HTTPConnection(parsed.hostname, parsed.port, timeout=4)
    try:
        connection.request(
            "POST",
            route,
            body=b"{}",
            headers={
                "Authorization": f"Bearer {bearer}",
                "Content-Type": "application/json",
                "Content-Length": "2",
            },
        )
        response = connection.getresponse()
        payload = response.read(1024 * 1024 + 1)
        if response.status != 200:
            raise RuntimeError(
                f"local synthetic preview route returned HTTP {response.status}"
            )
        if len(payload) > 1024 * 1024:
            raise RuntimeError("local synthetic preview response exceeded its byte limit")
        decoded = json.loads(payload)
        if not isinstance(decoded, dict):
            raise RuntimeError("local synthetic preview response was not an object")
        return decoded
    finally:
        connection.close()


def seed_market_history_pagination(binary: Path, home: Path) -> dict[str, dict[str, str]]:
    scenarios = {
        "baseline": "/api/v1/market/quote-preview",
        "alternate": "/api/v1/market/quote-preview/scenario/alternate",
    }
    digests: dict[str, dict[str, str]] = {}
    for scenario, route in scenarios.items():
        preview = authenticated_local_post(home, route)
        preview_digest = preview.get("digest")
        if not isinstance(preview_digest, str):
            raise RuntimeError("synthetic quote preview omitted its digest")

        approval_result = run_cli(
            binary,
            "market",
            "quote-approve",
            "--preview-digest",
            preview_digest,
            "--home",
            home,
        )
        require_success(approval_result, f"{scenario} synthetic quote approval")
        approval = json.loads(approval_result.stdout)
        deal_digest = approval.get("deal_digest")
        if not isinstance(deal_digest, str):
            raise RuntimeError(f"{scenario} approval omitted its deal digest")

        completion_result = run_cli(
            binary,
            "market",
            "quote-complete",
            "--preview-digest",
            preview_digest,
            "--deal-digest",
            deal_digest,
            "--home",
            home,
        )
        require_success(completion_result, f"{scenario} synthetic mock completion")
        completion = json.loads(completion_result.stdout)
        receipt_digest = completion["demo"]["receipt"].get("digest")
        if not isinstance(receipt_digest, str):
            raise RuntimeError(f"{scenario} mock completion omitted its receipt digest")
        digests[scenario] = {
            "deal_digest": deal_digest,
            "preview_digest": preview_digest,
            "receipt_digest": receipt_digest,
        }

    history_result = run_cli(
        binary, "market", "history", "--limit", "20", "--home", home
    )
    require_success(history_result, "two-record authenticated synthetic Market history")
    history = json.loads(history_result.stdout)
    entries = history["body"]["entries"]
    expected_deals = {item["deal_digest"] for item in digests.values()}
    observed_deals = {
        entry["approval"]["deal_digest"] for entry in entries
    }
    if len(entries) != 2 or observed_deals != expected_deals:
        raise RuntimeError("owner history did not contain the two distinct synthetic deals")
    for entry in entries:
        expected = next(
            item
            for item in digests.values()
            if item["deal_digest"] == entry["approval"]["deal_digest"]
        )
        receipt = entry["completion_receipt"]
        if (
            receipt is None
            or receipt["digest"] != expected["receipt_digest"]
            or entry["approval"]["preview_digest"] != expected["preview_digest"]
        ):
            raise RuntimeError("owner history changed a synthetic quote or receipt digest")
    return digests


class MarketHistoryPaginationProxy:
    """Forward test-home traffic and ask its real service for one history row per page."""

    def __init__(self, home: Path) -> None:
        self.home = home
        self.record_path = home / "runtime" / "service.json"
        self.original_record = self.record_path.read_bytes()
        record = json.loads(self.original_record)
        address = urlsplit(f"http://{record['address']}")
        if address.hostname != "127.0.0.1" or address.port is None:
            raise RuntimeError("pagination proxy upstream was not IPv4 loopback")
        self.upstream_host = address.hostname
        self.upstream_port = address.port
        self.history_requests = 0
        self.forwarded_cursor = False
        self.page_digests: list[str] = []
        self.error: str | None = None
        self.expected_cursor: str | None = None
        self.lock = threading.Lock()
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self) -> None:
                self.forward()

            def do_POST(self) -> None:
                self.forward()

            def log_message(self, _format: str, *args: object) -> None:
                return

            def forward(self) -> None:
                parsed = urlsplit(self.path)
                query = parse_qsl(parsed.query, keep_blank_values=True)
                is_history = (
                    self.command == "GET"
                    and parsed.path == "/api/v1/market/deal-history"
                )
                if is_history and not proxy.prepare_history_query(query):
                    self.send_error(502, "test history pagination request was invalid")
                    return

                target = urlunsplit(("", "", parsed.path, urlencode(query), ""))
                body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
                headers = {
                    name: self.headers[name]
                    for name in (
                        "Authorization",
                        "Accept",
                        "Content-Type",
                        "X-CSRF-Token",
                        "Idempotency-Key",
                    )
                    if self.headers.get(name) is not None
                }
                if body:
                    headers["Content-Length"] = str(len(body))

                upstream = http.client.HTTPConnection(
                    proxy.upstream_host, proxy.upstream_port, timeout=8
                )
                try:
                    upstream.request(self.command, target, body=body or None, headers=headers)
                    response = upstream.getresponse()
                    payload = response.read(1024 * 1024 + 1)
                    if len(payload) > 1024 * 1024:
                        proxy.set_error("upstream response exceeded the local test bound")
                        self.send_error(502, "local upstream response exceeded test bound")
                        return
                    if is_history and response.status == 200:
                        proxy.observe_history_response(payload)
                    self.send_response(response.status)
                    content_type = response.getheader("Content-Type")
                    if content_type is not None:
                        self.send_header("Content-Type", content_type)
                    self.send_header("Content-Length", str(len(payload)))
                    self.end_headers()
                    self.wfile.write(payload)
                except (OSError, http.client.HTTPException):
                    proxy.set_error("loopback service forwarding failed")
                    self.send_error(502, "local test service forwarding failed")
                finally:
                    upstream.close()

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        record["address"] = f"127.0.0.1:{self.server.server_port}"
        self.record_path.write_text(
            json.dumps(record, indent=2) + "\n", encoding="utf-8"
        )
        os.chmod(self.record_path, 0o600)

    def prepare_history_query(self, query: list[tuple[str, str]]) -> bool:
        with self.lock:
            self.history_requests += 1
            request_number = self.history_requests
            limits = [index for index, (name, _value) in enumerate(query) if name == "limit"]
            cursors = [value for name, value in query if name == "cursor"]
            if len(limits) != 1 or query[limits[0]][1] != "20":
                self.error = "unexpected TUI Market history limit"
                return False
            if request_number == 1:
                if cursors:
                    self.error = "first TUI Market history request included a cursor"
                    return False
            elif request_number == 2:
                if len(cursors) != 1 or cursors[0] != self.expected_cursor:
                    self.error = "TUI did not forward the exact service-issued cursor"
                    return False
                self.forwarded_cursor = True
            else:
                self.error = "TUI made an unexpected third Market history request"
                return False
            query[limits[0]] = ("limit", "1")
            return True

    def observe_history_response(self, payload: bytes) -> None:
        try:
            page = json.loads(payload)
            body = page["body"]
            entries = body["entries"]
            digest = page["digest"]
            if len(entries) != 1 or not isinstance(digest, str):
                raise ValueError
        except (KeyError, TypeError, ValueError):
            self.set_error("service returned an invalid one-record history page")
            return
        with self.lock:
            self.page_digests.append(digest)
            if len(self.page_digests) == 1:
                cursor = body.get("next_cursor")
                if not isinstance(cursor, str) or not cursor:
                    self.error = "first service page omitted its continuation cursor"
                else:
                    self.expected_cursor = cursor
            elif len(self.page_digests) == 2 and body.get("next_cursor") is not None:
                self.error = "second service page was not terminal"

    def set_error(self, message: str) -> None:
        with self.lock:
            self.error = message

    def report(self) -> dict[str, object]:
        with self.lock:
            return {
                "history_requests": self.history_requests,
                "forwarded_service_cursor": self.forwarded_cursor,
                "one_record_per_page": len(self.page_digests) == 2,
                "page_digests": list(self.page_digests),
                "error": self.error,
            }

    def restore_service_record(self) -> None:
        self.record_path.write_bytes(self.original_record)
        os.chmod(self.record_path, 0o600)

    def close(self) -> None:
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=5)
        if self.thread.is_alive():
            raise RuntimeError("test-owned Market history proxy did not stop")


def cursor_rows(transcript: bytes) -> list[int]:
    rows: list[int] = []
    for match in re.finditer(rb"\x1b\[([0-9;?]*)([Hf])", transcript):
        parameters = match.group(1).replace(b"?", b"").split(b";", maxsplit=1)[0]
        try:
            rows.append(int(parameters) if parameters else 1)
        except ValueError:
            continue
    return rows


class AnsiScreen:
    """Small in-memory screen model for Ratatui's cursor-addressed PTY output."""

    def __init__(self, rows: int, columns: int) -> None:
        self.rows = rows
        self.columns = columns
        self.cells = [[" "] * columns for _ in range(rows)]
        self.row = 0
        self.column = 0
        self.wrap_pending = False
        self.state = "ground"
        self.control = bytearray()
        self.utf8 = bytearray()
        self.utf8_length = 0
        self.saved_cursor = (0, 0)
        self.csi_sequences = 0
        self.alternate_screen_entries = 0

    def feed(self, data: bytes) -> None:
        for byte in data:
            self._consume(byte)

    def _consume(self, byte: int) -> None:
        if self.state == "csi":
            if 0x40 <= byte <= 0x7E:
                self.csi_sequences += 1
                self._execute_csi(chr(byte), bytes(self.control))
                self.control.clear()
                self.state = "ground"
            elif len(self.control) < 128:
                self.control.append(byte)
            else:
                self.control.clear()
                self.state = "ground"
            return

        if self.state == "string":
            if byte == 0x07:
                self.state = "ground"
            elif byte == 0x1B:
                self.state = "string_escape"
            return
        if self.state == "string_escape":
            self.state = "ground" if byte == ord("\\") else "string"
            return
        if self.state == "esc_intermediate":
            if 0x30 <= byte <= 0x7E:
                self.state = "ground"
            return
        if self.state == "esc":
            if byte == ord("["):
                self.control.clear()
                self.state = "csi"
            elif byte == ord("]") or byte in (ord("P"), ord("X"), ord("^"), ord("_")):
                self.state = "string"
            elif 0x20 <= byte <= 0x2F:
                self.state = "esc_intermediate"
            else:
                self._execute_esc(byte)
                self.state = "ground"
            return

        if self.utf8:
            if 0x80 <= byte <= 0xBF:
                self.utf8.append(byte)
                if len(self.utf8) == self.utf8_length:
                    try:
                        character = self.utf8.decode("utf-8")
                    except UnicodeDecodeError:
                        character = "\ufffd"
                    self._write_character(character)
                    self.utf8.clear()
                    self.utf8_length = 0
                return
            self._write_character("\ufffd")
            self.utf8.clear()
            self.utf8_length = 0
            self._consume(byte)
            return

        if byte == 0x1B:
            self.state = "esc"
        elif byte == 0x0D:
            self.column = 0
            self.wrap_pending = False
        elif byte == 0x0A:
            self._line_feed()
        elif byte == 0x08:
            self.column = max(0, self.column - 1)
            self.wrap_pending = False
        elif byte == 0x09:
            self.column = min(self.columns - 1, ((self.column // 8) + 1) * 8)
            self.wrap_pending = False
        elif byte < 0x20 or byte == 0x7F:
            return
        elif byte < 0x80:
            self._write_character(chr(byte))
        elif 0xC2 <= byte <= 0xDF:
            self.utf8 = bytearray([byte])
            self.utf8_length = 2
        elif 0xE0 <= byte <= 0xEF:
            self.utf8 = bytearray([byte])
            self.utf8_length = 3
        elif 0xF0 <= byte <= 0xF4:
            self.utf8 = bytearray([byte])
            self.utf8_length = 4
        else:
            self._write_character("\ufffd")

    def _execute_esc(self, byte: int) -> None:
        if byte == ord("7"):
            self.saved_cursor = (self.row, self.column)
        elif byte == ord("8"):
            self._set_cursor(*self.saved_cursor)
        elif byte == ord("D"):
            self._line_feed()
        elif byte == ord("E"):
            self._line_feed()
            self.column = 0
        elif byte == ord("M"):
            if self.row == 0:
                self.cells.insert(0, [" "] * self.columns)
                self.cells.pop()
            else:
                self.row -= 1
        elif byte == ord("c"):
            self.cells = [[" "] * self.columns for _ in range(self.rows)]
            self._set_cursor(0, 0)

    def _execute_csi(self, final: str, raw_parameters: bytes) -> None:
        parameters = raw_parameters.decode("ascii", errors="ignore")
        private = parameters.startswith("?")
        parameters = parameters.lstrip("?>!")
        parts = parameters.split(";") if parameters else []

        def parameter(index: int, default: int = 0) -> int:
            if index >= len(parts) or not parts[index]:
                return default
            try:
                return int(parts[index].split(":", maxsplit=1)[0])
            except ValueError:
                return default

        if final in ("H", "f"):
            self._set_cursor(max(0, parameter(0, 1) - 1), max(0, parameter(1, 1) - 1))
        elif final == "A":
            self._set_cursor(self.row - max(1, parameter(0, 1)), self.column)
        elif final == "B":
            self._set_cursor(self.row + max(1, parameter(0, 1)), self.column)
        elif final == "C":
            self._set_cursor(self.row, self.column + max(1, parameter(0, 1)))
        elif final == "D":
            self._set_cursor(self.row, self.column - max(1, parameter(0, 1)))
        elif final == "E":
            self._set_cursor(self.row + max(1, parameter(0, 1)), 0)
        elif final == "F":
            self._set_cursor(self.row - max(1, parameter(0, 1)), 0)
        elif final == "G":
            self._set_cursor(self.row, max(0, parameter(0, 1) - 1))
        elif final == "d":
            self._set_cursor(max(0, parameter(0, 1) - 1), self.column)
        elif final == "J":
            self._erase_display(parameter(0))
        elif final == "K":
            self._erase_line(parameter(0))
        elif final == "s":
            self.saved_cursor = (self.row, self.column)
        elif final == "u":
            self._set_cursor(*self.saved_cursor)
        elif final == "h" and private and any(part == "1049" for part in parts):
            self.alternate_screen_entries += 1
            self.cells = [[" "] * self.columns for _ in range(self.rows)]
            self._set_cursor(0, 0)
        elif final == "S":
            for _ in range(min(self.rows, max(1, parameter(0, 1)))):
                self.cells.pop(0)
                self.cells.append([" "] * self.columns)
        elif final == "T":
            for _ in range(min(self.rows, max(1, parameter(0, 1)))):
                self.cells.pop()
                self.cells.insert(0, [" "] * self.columns)

    def _set_cursor(self, row: int, column: int) -> None:
        self.row = min(self.rows - 1, max(0, row))
        self.column = min(self.columns - 1, max(0, column))
        self.wrap_pending = False

    def _line_feed(self) -> None:
        if self.row == self.rows - 1:
            self.cells.pop(0)
            self.cells.append([" "] * self.columns)
        else:
            self.row += 1
        self.wrap_pending = False

    def _write_character(self, character: str) -> None:
        if not character:
            return
        if unicodedata.combining(character):
            previous = self.column - 1 if not self.wrap_pending else self.column
            if previous >= 0:
                self.cells[self.row][previous] += character
            return
        width = 2 if unicodedata.east_asian_width(character) in ("F", "W") else 1
        if self.wrap_pending or self.column + width > self.columns:
            self._line_feed()
            self.column = 0
        self.cells[self.row][self.column] = character
        if width == 2 and self.column + 1 < self.columns:
            self.cells[self.row][self.column + 1] = ""
        next_column = self.column + width
        if next_column >= self.columns:
            self.column = self.columns - 1
            self.wrap_pending = True
        else:
            self.column = next_column

    def _erase_line(self, mode: int) -> None:
        if mode == 0:
            start, end = self.column, self.columns
        elif mode == 1:
            start, end = 0, self.column + 1
        elif mode == 2:
            start, end = 0, self.columns
        else:
            return
        self.cells[self.row][start:end] = [" "] * (end - start)

    def _erase_display(self, mode: int) -> None:
        if mode == 2 or mode == 3:
            self.cells = [[" "] * self.columns for _ in range(self.rows)]
        elif mode == 0:
            self._erase_line(0)
            for row in range(self.row + 1, self.rows):
                self.cells[row] = [" "] * self.columns
        elif mode == 1:
            self._erase_line(1)
            for row in range(0, self.row):
                self.cells[row] = [" "] * self.columns

    def visible_text(self) -> str:
        return "\n".join("".join(row).rstrip() for row in self.cells)

    def contains(self, marker: str) -> bool:
        normalize = lambda text: re.sub(r"\s+", " ", text).strip()
        return normalize(marker) in normalize(self.visible_text())

    def safe_diagnostics(self, marker: str, transcript: bytes) -> dict[str, object]:
        fixed_markers = (
            "Workspace:",
            "Overview",
            "Workspace: Market / buyer supply",
            "SYNTHETIC QUOTE PREVIEW",
            "SYNTHETIC BUYER APPROVAL",
            "MOCK FULFILLMENT AND RECEIPT",
            "Receipt digest:",
            "Mock fulfillment and receipt completed.",
            "Mock completion failed:",
            "End of the available local history pages.",
            "No additional local history page is available.",
            "Press c to request the deterministic mock receipt",
            "local_synthetic_mock_fulfillment_verifier_and_settlement_only",
        )
        return {
            "expected_marker_visible": self.contains(marker),
            "fixed_marker_presence": {
                fixed_marker: self.contains(fixed_marker)
                for fixed_marker in fixed_markers
            },
            "receipt_digest_visible": contains_visible_sha256_after(
                self.visible_text(), "Receipt digest:"
            ),
            "csi_sequences": self.csi_sequences,
            "alternate_screen_entries": self.alternate_screen_entries,
            "transcript_bytes": len(transcript),
            "transcript_sha256": hashlib.sha256(transcript).hexdigest(),
        }


def collect_pty_output(
    master: int,
    child: int,
    transcript: bytearray,
    duration: float,
    screen: AnsiScreen | None = None,
) -> None:
    deadline = time.monotonic() + duration
    while time.monotonic() < deadline:
        remaining = deadline - time.monotonic()
        readable, _, _ = select.select([master], [], [], min(0.05, remaining))
        if readable:
            try:
                chunk = os.read(master, 65536)
            except OSError as error:
                raise RuntimeError("PTY became unreadable during the resize check") from error
            if not chunk:
                raise RuntimeError("PTY closed during the resize check")
            transcript.extend(chunk)
            if screen is not None:
                screen.feed(chunk)
        finished_pid, status = os.waitpid(child, os.WNOHANG)
        if finished_pid:
            raise RuntimeError(
                f"desk exited with status {os.waitstatus_to_exitcode(status)} during resize"
            )


def wait_for_screen_marker(
    master: int,
    child: int,
    transcript: bytearray,
    screen: AnsiScreen,
    marker: str,
    timeout: float,
) -> None:
    wait_for_screen_condition(
        master,
        child,
        transcript,
        screen,
        marker,
        lambda current_screen: current_screen.contains(marker),
        timeout,
    )


def wait_for_screen_condition(
    master: int,
    child: int,
    transcript: bytearray,
    screen: AnsiScreen,
    description: str,
    condition: Callable[[AnsiScreen], bool],
    timeout: float,
) -> None:
    deadline = time.monotonic() + timeout
    while not condition(screen):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            diagnostics = screen.safe_diagnostics(description, transcript)
            raise RuntimeError(
                "desk did not satisfy expected screen condition "
                f"{description!r} ({json.dumps(diagnostics, sort_keys=True)})"
            )
        readable, _, _ = select.select([master], [], [], min(0.05, remaining))
        if readable:
            try:
                chunk = os.read(master, 65536)
            except OSError as error:
                raise RuntimeError("PTY became unreadable during the quote lifecycle") from error
            if not chunk:
                raise RuntimeError("PTY closed during the quote lifecycle")
            transcript.extend(chunk)
            screen.feed(chunk)
        finished_pid, status = os.waitpid(child, os.WNOHANG)
        if finished_pid:
            raise RuntimeError(
                "desk exited with status "
                f"{os.waitstatus_to_exitcode(status)} during the quote lifecycle"
            )


def exercise_market_quote_lifecycle(
    master: int,
    child: int,
    transcript: bytearray,
) -> dict[str, bool]:
    screen = AnsiScreen(40, 120)
    screen.feed(transcript)
    overview_marker = "Workspace: Overview"
    overview_marker_preobserved = screen.contains(overview_marker)
    wait_for_screen_marker(master, child, transcript, screen, overview_marker, 5)
    collect_pty_output(master, child, transcript, 0.3, screen)

    os.write(master, b"j")
    wait_for_screen_marker(
        master, child, transcript, screen, "Workspace: Market / buyer supply", 5
    )

    os.write(master, b"\r")
    wait_for_screen_marker(
        master, child, transcript, screen, "SYNTHETIC QUOTE PREVIEW", 5
    )
    preview_rendered = screen.contains("SYNTHETIC QUOTE PREVIEW")

    os.write(master, b"a")
    wait_for_screen_marker(
        master, child, transcript, screen, "SYNTHETIC BUYER APPROVAL", 5
    )
    approval_rendered = screen.contains("SYNTHETIC BUYER APPROVAL")

    os.write(master, b"c")
    wait_for_screen_condition(
        master,
        child,
        transcript,
        screen,
        "visible 64-character receipt digest",
        lambda current_screen: contains_visible_sha256_after(
            current_screen.visible_text(), "Receipt digest:"
        ),
        5,
    )

    completed_output = re.sub(r"\s+", " ", screen.visible_text())
    return {
        "overview_rendered": True,
        "overview_marker_preobserved": overview_marker_preobserved,
        "market_selected": True,
        "preview_rendered": preview_rendered,
        "approval_rendered": approval_rendered,
        "receipt_rendered": screen.contains("MOCK FULFILLMENT AND RECEIPT"),
        "receipt_digest_displayed": contains_visible_sha256_after(
            completed_output, "Receipt digest:"
        ),
        "synthetic_effect_boundary_displayed": (
            "local_synthetic_mock_fulfillment_verifier_and_settlement_only"
            in completed_output
        ),
    }


def exercise_market_history(
    master: int,
    child: int,
    transcript: bytearray,
    digests: dict[str, str],
) -> dict[str, bool]:
    screen = AnsiScreen(40, 120)
    screen.feed(transcript)
    wait_for_screen_marker(master, child, transcript, screen, "Workspace: Overview", 5)
    os.write(master, b"jj")
    wait_for_screen_marker(
        master,
        child,
        transcript,
        screen,
        "Workspace: Deals / Market-owned local history",
        5,
    )
    wait_for_screen_condition(
        master,
        child,
        transcript,
        screen,
        "seeded mock-completed history row",
        lambda current_screen: (
            current_screen.contains(digests["deal_digest"])
            and current_screen.contains(digests["preview_digest"])
            and current_screen.contains("MOCK COMPLETED")
            and contains_visible_sha256_after(
                current_screen.visible_text(), "Mock receipt digest:"
            )
        ),
        5,
    )
    wait_for_screen_marker(
        master,
        child,
        transcript,
        screen,
        "End of the available local history pages.",
        5,
    )
    initial_page = screen.visible_text()
    if "no execution or payment claim" not in re.sub(r"\s+", " ", initial_page):
        raise RuntimeError("Deals did not display the synthetic claim boundary")
    if "End of the available local history pages." not in re.sub(
        r"\s+", " ", initial_page
    ):
        raise RuntimeError("Deals did not display the terminal-page state")

    os.write(master, b"n")
    wait_for_screen_marker(
        master,
        child,
        transcript,
        screen,
        "No additional local history page is available.",
        5,
    )
    return {
        "deals_workspace_rendered": True,
        "seeded_deal_digest_visible": screen.contains(digests["deal_digest"]),
        "seeded_preview_digest_visible": screen.contains(digests["preview_digest"]),
        "mock_completed_label_visible": screen.contains("MOCK COMPLETED"),
        "mock_receipt_digest_visible": contains_visible_sha256_after(
            screen.visible_text(), "Mock receipt digest:"
        ),
        "claim_boundary_visible": True,
        "terminal_page_visible": True,
        "no_additional_page_feedback_visible": True,
    }


def exercise_market_history_pagination(
    master: int,
    child: int,
    transcript: bytearray,
    digests: dict[str, dict[str, str]],
    proxy: MarketHistoryPaginationProxy,
) -> dict[str, object]:
    screen = AnsiScreen(40, 120)
    screen.feed(transcript)
    wait_for_screen_marker(master, child, transcript, screen, "Workspace: Overview", 5)
    os.write(master, b"jj")
    wait_for_screen_marker(
        master,
        child,
        transcript,
        screen,
        "Workspace: Deals / Market-owned local history",
        5,
    )
    alternate = digests["alternate"]
    baseline = digests["baseline"]
    wait_for_screen_condition(
        master,
        child,
        transcript,
        screen,
        "first authenticated history page",
        lambda current_screen: (
            current_screen.contains(alternate["deal_digest"])
            and current_screen.contains(alternate["preview_digest"])
            and current_screen.contains(alternate["receipt_digest"])
            and not current_screen.contains(baseline["deal_digest"])
            and current_screen.contains("More local records are available.")
        ),
        5,
    )
    first_page = re.sub(r"\s+", " ", screen.visible_text())
    if "no execution or payment claim" not in first_page:
        raise RuntimeError("first Deals page omitted the synthetic claim boundary")
    if "End of the available local history pages." in first_page:
        raise RuntimeError("first Deals page was incorrectly presented as terminal")

    os.write(master, b"n")
    wait_for_screen_condition(
        master,
        child,
        transcript,
        screen,
        "second authenticated history page",
        lambda current_screen: (
            current_screen.contains(alternate["deal_digest"])
            and current_screen.contains(alternate["preview_digest"])
            and current_screen.contains(alternate["receipt_digest"])
            and current_screen.contains(baseline["deal_digest"])
            and current_screen.contains(baseline["preview_digest"])
            and current_screen.contains(baseline["receipt_digest"])
            and current_screen.contains("End of the available local history pages.")
        ),
        5,
    )
    compact_visible = re.sub(r"\s+", "", screen.visible_text())
    for page_digest in proxy.report()["page_digests"]:
        if not isinstance(page_digest, str) or page_digest not in compact_visible:
            raise RuntimeError("TUI did not render both verified service page digests")

    os.write(master, b"n")
    wait_for_screen_marker(
        master,
        child,
        transcript,
        screen,
        "No additional local history page is available.",
        5,
    )
    time.sleep(0.2)
    proxy_report = proxy.report()
    if (
        proxy_report["history_requests"] != 2
        or not proxy_report["forwarded_service_cursor"]
        or not proxy_report["one_record_per_page"]
        or proxy_report["error"] is not None
    ):
        raise RuntimeError("TUI history requests did not follow the two service pages")
    return {
        "deals_workspace_rendered": True,
        "first_page_alternate_record_visible": True,
        "second_page_baseline_record_visible": True,
        "both_preview_and_receipt_digests_visible": True,
        "both_service_page_digests_visible": True,
        "terminal_page_visible": True,
        "no_additional_page_feedback_visible": True,
        "history_requests": proxy_report["history_requests"],
        "forwarded_service_cursor": proxy_report["forwarded_service_cursor"],
    }


def wait_for_resize_redraw(
    master: int,
    child: int,
    transcript: bytearray,
    rows: int,
    columns: int,
    navigation_keys: bytes,
    capture_seconds: float,
    minimum_cursor_row: int,
) -> dict[str, int]:
    start = len(transcript)
    fcntl.ioctl(
        master,
        termios.TIOCSWINSZ,
        struct.pack("HHHH", rows, columns, 0, 0),
    )
    actual_rows, actual_columns, _, _ = struct.unpack(
        "HHHH", fcntl.ioctl(master, termios.TIOCGWINSZ, b"\0" * 8)
    )
    if (actual_rows, actual_columns) != (rows, columns):
        raise RuntimeError(
            f"PTY resize requested {columns}x{rows} but reports {actual_columns}x{actual_rows}"
    )
    collect_pty_output(master, child, transcript, 0.3)
    start = len(transcript)
    os.write(master, navigation_keys)
    collect_pty_output(master, child, transcript, capture_seconds)
    redraw = bytes(transcript[start:])
    positions = cursor_rows(redraw)
    if not redraw or not positions:
        raise RuntimeError(f"desk emitted no cursor-positioned redraw after resize to {columns}x{rows}")
    maximum_row = max(positions)
    if maximum_row > rows:
        raise RuntimeError(
            f"desk addressed terminal row {maximum_row} after resize to height {rows}"
        )
    if maximum_row < minimum_cursor_row:
        raise RuntimeError(
            f"desk addressed no newly exposed rows after resize to height {rows}"
        )
    return {
        "columns": columns,
        "rows": rows,
        "navigation_events": len(navigation_keys),
        "redraw_bytes": len(redraw),
        "maximum_cursor_row": maximum_row,
    }


def run_desk(
    binary: Path,
    home: Path,
    expect_panic: bool = False,
    market_quote_lifecycle: bool = False,
    market_history_digests: dict[str, str] | None = None,
    market_history_pagination_digests: dict[str, dict[str, str]] | None = None,
    market_history_proxy: MarketHistoryPaginationProxy | None = None,
) -> dict[str, object]:
    master, slave = pty.openpty()
    child: int | None = None
    transcript = bytearray()
    try:
        initial_rows, initial_columns = (
            (40, 120)
            if (
                market_quote_lifecycle
                or market_history_digests is not None
                or market_history_pagination_digests is not None
            )
            else (24, 80)
        )
        fcntl.ioctl(
            slave,
            termios.TIOCSWINSZ,
            struct.pack("HHHH", initial_rows, initial_columns, 0, 0),
        )
        baseline = termios.tcgetattr(slave)
        child = os.fork()
        if child == 0:
            try:
                for descriptor in (0, 1, 2):
                    os.dup2(slave, descriptor)
                if slave > 2:
                    os.close(slave)
                os.close(master)
                if expect_panic:
                    os.environ["HYPERAI_TUI_TEST_PANIC_AFTER_DRAW"] = "1"
                os.execv(binary, [str(binary), "desk", "--home", str(home)])
            except BaseException:
                os._exit(127)

        started_at = time.monotonic()
        alternate_screen_entered = False
        exit_code: int | None = None
        while time.monotonic() - started_at < 8:
            readable, _, _ = select.select([master], [], [], 0.1)
            if readable:
                try:
                    transcript.extend(os.read(master, 65536))
                except OSError:
                    pass
                if b"\x1b[?1049h" in transcript:
                    alternate_screen_entered = True
                    break
            finished_pid, status = os.waitpid(child, os.WNOHANG)
            if finished_pid:
                exit_code = os.waitstatus_to_exitcode(status)
                break
        if not alternate_screen_entered:
            raise RuntimeError("desk did not enter the alternate screen within 8 seconds")

        raw_mode_entered = termios.tcgetattr(slave) != baseline
        if not raw_mode_entered:
            raise RuntimeError("desk did not enable terminal raw mode")

        quote_report = None
        if market_quote_lifecycle:
            quote_report = exercise_market_quote_lifecycle(master, child, transcript)
        market_history_report = None
        if market_history_digests is not None:
            market_history_report = exercise_market_history(
                master, child, transcript, market_history_digests
            )
        market_history_pagination_report = None
        if market_history_pagination_digests is not None:
            if market_history_proxy is None:
                raise RuntimeError("Market history pagination proxy was not started")
            market_history_pagination_report = exercise_market_history_pagination(
                master,
                child,
                transcript,
                market_history_pagination_digests,
                market_history_proxy,
            )

        if expect_panic:
            os.write(master, b"p")
            stop_deadline = time.monotonic() + 10
            terminal_restored = False
            while time.monotonic() < stop_deadline:
                try:
                    terminal_restored = termios.tcgetattr(slave) == baseline
                except OSError:
                    terminal_restored = False

                readable, _, _ = select.select([master], [], [], 0.05)
                if readable:
                    try:
                        transcript.extend(os.read(master, 65536))
                    except OSError:
                        pass

                finished_pid, status = os.waitpid(child, os.WNOHANG)
                if finished_pid:
                    exit_code = os.waitstatus_to_exitcode(status)
                    break

            if exit_code is None:
                raise RuntimeError("desk did not unwind within 10 seconds after the test key")
            if exit_code != 101:
                raise RuntimeError(f"desk exited with status {exit_code}, expected Rust panic status 101")
            if not terminal_restored and termios.tcgetattr(slave) == baseline:
                terminal_restored = True
            if not terminal_restored:
                raise RuntimeError("terminal settings did not return to their initial value after panic")
            if b"test-support panic after first TUI draw" not in transcript:
                raise RuntimeError("the expected test-support panic was not observed in the PTY")
            if b"\x1b[?1049l" not in transcript:
                raise RuntimeError("alternate-screen exit sequence was not captured after panic")
            if b"\x1b[?25h" not in transcript:
                raise RuntimeError("cursor restoration sequence was not captured after panic")

            return {
                "alternate_screen_entered": True,
                "raw_mode_entered": True,
                "panic_exit": exit_code,
                "panic_observed": True,
                "termios_restored": True,
                "alternate_screen_left": True,
                "cursor_restored": True,
                "transcript_bytes": len(transcript),
                "transcript_sha256": hashlib.sha256(transcript).hexdigest(),
            }

        collect_pty_output(master, child, transcript, 0.3)

        resize_checks = [
            wait_for_resize_redraw(master, child, transcript, 10, 48, b"j" * 11, 1.5, 1),
            wait_for_resize_redraw(master, child, transcript, 30, 100, b"j", 0.4, 11),
        ]

        os.write(master, b"\x03")
        stop_deadline = time.monotonic() + 10
        terminal_restored = False
        while time.monotonic() < stop_deadline:
            try:
                terminal_restored = termios.tcgetattr(slave) == baseline
            except OSError:
                terminal_restored = False

            readable, _, _ = select.select([master], [], [], 0.05)
            if readable:
                try:
                    transcript.extend(os.read(master, 65536))
                except OSError:
                    pass

            finished_pid, status = os.waitpid(child, os.WNOHANG)
            if finished_pid:
                exit_code = os.waitstatus_to_exitcode(status)
                break

        if exit_code is None:
            raise RuntimeError("desk did not exit within 10 seconds after Ctrl+C")
        if exit_code != 0:
            raise RuntimeError(f"desk exited with status {exit_code} after Ctrl+C")
        if not terminal_restored and termios.tcgetattr(slave) == baseline:
            terminal_restored = True
        if not terminal_restored:
            raise RuntimeError("terminal settings did not return to their initial value")
        if b"\x1b[?1049l" not in transcript:
            raise RuntimeError("alternate-screen exit sequence was not captured")
        if b"\x1b[?25h" not in transcript:
            raise RuntimeError("cursor restoration sequence was not captured")

        report: dict[str, object] = {
            "alternate_screen_entered": True,
            "raw_mode_entered": True,
            "ctrl_c_exit": exit_code,
            "termios_restored": True,
            "alternate_screen_left": True,
            "cursor_restored": True,
            "resize_checks": resize_checks,
            "transcript_bytes": len(transcript),
            "transcript_sha256": hashlib.sha256(transcript).hexdigest(),
        }
        if quote_report is not None:
            report["market_quote_lifecycle"] = quote_report
        if market_history_report is not None:
            report["market_history_service"] = market_history_report
        if market_history_pagination_report is not None:
            report["market_history_pagination"] = market_history_pagination_report
        return report
    finally:
        if child is not None:
            try:
                finished_pid, _ = os.waitpid(child, os.WNOHANG)
                if finished_pid == 0:
                    os.kill(child, signal.SIGKILL)
                    os.waitpid(child, 0)
            except ChildProcessError:
                pass
        os.close(master)
        os.close(slave)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    repository = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path("target/debug/hyperai"),
        help="already-built hyperai binary (default: target/debug/hyperai)",
    )
    parser.add_argument(
        "--expect-panic",
        action="store_true",
        help="enable the feature-gated panic hook and verify live terminal restoration",
    )
    parser.add_argument(
        "--market-quote-lifecycle",
        action="store_true",
        help="exercise the local synthetic Market quote flow in the installed TUI",
    )
    parser.add_argument(
        "--market-history",
        action="store_true",
        help="exercise populated Market history from the authenticated service in the TUI",
    )
    parser.add_argument(
        "--market-history-pagination",
        action="store_true",
        help="exercise two authenticated Market history pages in the installed TUI",
    )
    arguments = parser.parse_args()
    if sum(
        (
            arguments.market_quote_lifecycle,
            arguments.market_history,
            arguments.market_history_pagination,
        )
    ) > 1:
        parser.error("quote-lifecycle and Market history PTY modes are separate runs")
    if os.name != "posix":
        parser.error("this standard-library PTY harness currently supports POSIX hosts only")

    binary = arguments.binary
    if not binary.is_absolute():
        binary = repository / binary
    binary = binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"executable HyperAI binary not found: {binary}")

    with tempfile.TemporaryDirectory(prefix="hyperai-ctrlc-") as temporary:
        home = Path(temporary) / "home"
        require_success(run_cli(binary, "init", "--home", home), "home initialization")
        require_success(
            run_cli(binary, "service", "start", "--home", home, "--port", "0"),
            "local service startup",
        )
        try:
            market_history_digests = (
                seed_market_history(binary, home) if arguments.market_history else None
            )
            market_history_pagination_digests = (
                seed_market_history_pagination(binary, home)
                if arguments.market_history_pagination
                else None
            )
            proxy = (
                MarketHistoryPaginationProxy(home)
                if market_history_pagination_digests is not None
                else None
            )
            try:
                terminal = run_desk(
                    binary,
                    home,
                    expect_panic=arguments.expect_panic,
                    market_quote_lifecycle=arguments.market_quote_lifecycle,
                    market_history_digests=market_history_digests,
                    market_history_pagination_digests=market_history_pagination_digests,
                    market_history_proxy=proxy,
                )
                if proxy is not None:
                    terminal["market_history_pagination_proxy"] = proxy.report()
            finally:
                if proxy is not None:
                    try:
                        proxy.restore_service_record()
                    finally:
                        proxy.close()
        finally:
            require_success(run_cli(binary, "service", "stop", "--home", home), "service shutdown")

    print(
        json.dumps(
            {
                "platform": f"{sys.platform} {os.uname().machine}",
                "python": sys.version.split()[0],
                "binary_sha256": sha256_file(binary),
                "temporary_home_removed": not home.exists(),
                **terminal,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"HyperAI TUI Ctrl+C check failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
