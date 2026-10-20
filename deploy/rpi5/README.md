# Raspberry Pi 5 deployment: HyperAI live GPU price board

State slice: `hyperai.demo.live_price_board_static_export.v1`. Claim ceiling:
bounded local replay. Public prices from an aggregator, read-only.

## Architecture (no public attack surface beyond static files)

```
internet -> Caddy (static files only, GET/HEAD) -> /srv/hyperai-board/{index.html,board.js,board.css,live-board.json}
                                                         ^ written atomically every 6h
systemd timer -> export.mjs (node) --loopback+bearer--> hyperai service (127.0.0.1:4317, owner-only)
                                                         -> one GET https://gpudiff.com/api/v1/offers.json
```

The owner service is never reachable from outside. Its refresh/schedule/deal/fleet routes
are therefore never exposed. Only the static JSON the exporter derives is public.

## 1. Build the binary (on your Mac, Docker running)

```sh
docker buildx build --platform linux/arm64 -f deploy/rpi5/Dockerfile.build \
  --output type=local,dest=dist/rpi5 .
file dist/rpi5/hyperai    # expect: ELF 64-bit ... ARM aarch64
```
Or build natively on the Pi with the pinned Rust 1.93.1 (slow: DataFusion).

## 2. Install on the Pi (Raspberry Pi OS 64-bit, Node 22+, Caddy)

```sh
sudo useradd --system --home /var/lib/hyperai --shell /usr/sbin/nologin hyperai
sudo install -d -o hyperai -g hyperai -m 700 /var/lib/hyperai
sudo install -d -o hyperai -g hyperai -m 755 /srv/hyperai-board
sudo install -d /opt/hyperai/bin /opt/hyperai/demo
sudo install -m 755 hyperai /opt/hyperai/bin/hyperai
sudo install -m 644 demo/live-board/export.mjs /opt/hyperai/demo/export.mjs
sudo install -m 644 demo/live-board/site/{index.html,board.css,board.js} /srv/hyperai-board/
sudo -u hyperai /opt/hyperai/bin/hyperai init --home /var/lib/hyperai
sudo install -m 644 deploy/rpi5/hyperai-*.service deploy/rpi5/hyperai-*.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now hyperai-service.service hyperai-live-board.timer
sudo systemctl start hyperai-live-board.service     # first export now
```
Caddy: copy `Caddyfile` to `/etc/caddy/Caddyfile`, set your hostname, `sudo systemctl reload caddy`.
Behind Cloudflare Tunnel / existing reverse proxy instead: point it at the static root only.

## 3. Verify

```sh
curl -s https://board.example.com/live-board.json | head
journalctl -u hyperai-live-board.service -n 20
curl -sI -X POST https://board.example.com/        # expect 405
```

## Operations and limits
- Stale data: the page warns after 24 h without a successful export; the last good file stays up.
- Retention: the data layer keeps at most 16 distinct source bodies and fails closed at capacity;
  clear old snapshots under `/var/lib/hyperai` deliberately if refreshes start failing.
- Attribution (CC BY 4.0, cite gpudiff.com) is on the page; keep it.
- Not tested on a Pi in this repo: the cross-build, unit files, and Caddy config are provided
  but unverified on hardware. The exporter and site are tested locally (`pnpm run test:demo`).
