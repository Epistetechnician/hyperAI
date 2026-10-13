from __future__ import annotations

import re
import unittest

from scripts.verify_hyperai_tui_ctrl_c import AnsiScreen, contains_visible_sha256_after


class AnsiScreenTests(unittest.TestCase):
    def setUp(self) -> None:
        self.screen = AnsiScreen(8, 40)

    def test_split_csi_and_utf8_sequences_render_across_chunks(self) -> None:
        self.screen.feed(b"\x1b[?1049h\x1b[2J\x1b[4;")
        self.screen.feed(b"1HWorkspace: Overview \xe2")
        self.screen.feed(b"\x80")
        self.screen.feed(b"\x94 ready")
        self.assertTrue(self.screen.contains("Workspace: Overview"))
        self.assertTrue(self.screen.contains("Overview — ready"))
        self.assertEqual(self.screen.alternate_screen_entries, 1)

    def test_cursor_positioned_frame_diff_and_clear_line_replace_old_text(self) -> None:
        self.screen.feed(b"\x1b[?1049h\x1b[3;1HWorkspace: Overview")
        self.screen.feed(b"\x1b[3;1H\x1b[KWorkspace: Market / buyer supply")
        self.assertTrue(self.screen.contains("Workspace: Market / buyer supply"))
        self.assertFalse(self.screen.contains("Workspace: Overview"))

    def test_visible_text_normalizes_terminal_line_wrapping(self) -> None:
        self.screen.feed(b"\x1b[2;1Hreceipt:\r\ndigest")
        self.assertTrue(self.screen.contains("receipt: digest"))

    def test_wrapped_receipt_digest_is_recognized_across_screen_rows(self) -> None:
        screen = AnsiScreen(8, 120)
        receipt_digest = "b" * 64
        screen.feed(
            (
                f"\x1b[1;1HDeal: {'a' * 64} · Receipt digest: {receipt_digest}\r\n"
                "Boundary: synthetic"
            ).encode()
        )
        visible_text = screen.visible_text()
        rows = visible_text.splitlines()
        self.assertTrue(any(row.endswith(receipt_digest[:31]) for row in rows))
        self.assertTrue(any(row.startswith(receipt_digest[31:]) for row in rows))
        self.assertFalse(
            re.search(
                r"Receipt digest: [0-9a-f]{64}",
                re.sub(r"\s+", " ", visible_text),
            )
        )
        self.assertTrue(
            contains_visible_sha256_after(visible_text, "Receipt digest:")
        )

    def test_wrapped_digest_match_rejects_short_nonhex_and_overlong_values(self) -> None:
        for digest in ["a" * 63, "a" * 63 + "g", "a" * 65]:
            with self.subTest(digest_length=len(digest), final=digest[-1]):
                self.assertFalse(
                    contains_visible_sha256_after(
                        f"Receipt digest: {digest}", "Receipt digest:"
                    )
                )

    def test_receipt_heading_chunk_does_not_complete_digest_wait(self) -> None:
        screen = AnsiScreen(8, 120)
        receipt_digest = "c" * 64
        screen.feed(
            b"\x1b[1;1HMOCK FULFILLMENT AND RECEIPT\r\nReceipt digest: "
            + receipt_digest[:32].encode()
        )
        self.assertTrue(screen.contains("MOCK FULFILLMENT AND RECEIPT"))
        self.assertFalse(
            contains_visible_sha256_after(screen.visible_text(), "Receipt digest:")
        )

        screen.feed(receipt_digest[32:].encode())
        self.assertTrue(
            contains_visible_sha256_after(screen.visible_text(), "Receipt digest:")
        )

    def test_osc_string_terminator_does_not_hide_following_screen_text(self) -> None:
        self.screen.feed(b"\x1b]0;terminal title\x1b\\")
        self.screen.feed(b"\x1b[1;1Hvisible")
        self.assertTrue(self.screen.contains("visible"))


if __name__ == "__main__":
    unittest.main()
