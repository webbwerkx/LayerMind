# LayerMind — Complete Step-by-Step Tutorial

**From nothing to running diagnostics on your printer.**

---

## Step 0: What You Need

- A Klipper-based 3D printer with Moonraker running (any Voron, Creality, Prusa, etc.)
- The printer's hostname or IP address
- Rust installed (covered below)

---

## Step 1: Install Rust (if you haven't already)

Open a terminal and paste this:

```fish
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

When it asks, choose **1) Proceed with installation (default)**. After it finishes:

```fish
# Close this terminal and open a new one, or reload your config:
source ~/.config/fish/config.fish

# Verify:
rustc --version
# Should show: rustc 1.85.0 (or newer)
```

---

## Step 2: Clone LayerMind

```fish
cd ~/dev
git clone git@github.com:webbwerkx/LayerMind.git
cd LayerMind
```

---

## Step 3: Find Your Printer's Address

You need Moonraker's WebSocket URL. This is the address your printer's web interface (Mainsail/Fluidd) runs on, but with `ws://` instead of `http://`.

**Method 1 — from Mainsail/Fluidd:**
- Open Mainsail/Fluidd in your browser
- Look at the URL bar — it'll be something like `http://voron-0.local:7125` or `http://192.168.1.100:7125`
- Your WebSocket URL is: `ws://voron-0.local:7125/websocket`

**Method 2 — guess common hostnames:**
```fish
ping voron-0.local
ping klipperpi.local
ping printerpi.local
# Pick the one that responds
```

**Method 3 — check your router:**
Open your router's admin page (usually 192.168.1.1), look at DHCP leases, find your printer.

**Once you have it, verify it works:**
```fish
curl http://voron-0.local:7125/printer/info
```
You should get a JSON response with printer info. If you get "connection refused", the address is wrong.

---

## Step 4: Set Everything Permanently

Replace `voron-0.local` with your actual printer address:

```fish
echo 'set -x MOONRAKER_URL ws://voron-0.local:7125/websocket' >> ~/.config/fish/config.fish
echo 'set -x LAYERMIND_PROVIDER openrouter' >> ~/.config/fish/config.fish
echo 'set -x LAYERMIND_MODEL deepseek/deepseek-chat' >> ~/.config/fish/config.fish
```

Then reload:

```fish
source ~/.config/fish/config.fish
```

Now these variables are set every time you open a terminal. You never need to type them again.

---

## Step 5: Build LayerMind

```fish
cd ~/dev/LayerMind
cargo build --release
```

This takes 2-5 minutes the first time (downloading and compiling dependencies). Subsequent builds are seconds.

---

## Step 6: Install the Binaries

```fish
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/

# Verify they work:
layermind --version
layermind --help
```

---

## Step 7: Test the Connection

```fish
layermind printer test
```

**What should happen:** It connects to your printer, queries Moonraker, and prints a full hardware report:
```
Moonraker Connection
  URL: ws://voron-0.local:7125/websocket

Machine:
  Motion: CoreXY
  Hostname: voron-0
  Klipper: v0.12.0
  Manufacturer: LDO
  Model: Voron 2.4

Hardware:
  Extruders: 1  Hotends: 1  MCUs: 2  Fans: 3  Probes: 1
  Axes: 3  Build volume: 350 × 350 × 350 mm

Capabilities:
  Input shaping: supported
  Pressure advance: supported
  BLTouch/CRTouch: supported
  ...
```

**If it fails:** Double-check your `MOONRAKER_URL` — the address or port may be wrong.

---

## Step 8: Start the Daemon

This runs the full pipeline in the background:

```fish
layermind run
```

You'll see:
```
2026-...  INFO  layermind_core: LayerMind starting
2026-...  INFO  layermind_core: telemetry engine ready
2026-...  INFO  layermind_core: all services started — pipeline active
2026-...  INFO  layermind_core: machine profile discovered and stored
```

**Leave this running.** Open a **second terminal window** for the next steps.

To stop it later: press **Ctrl+C** in this terminal.

---

## Step 9: Check Printer Status

In terminal #2 (with the daemon still running in terminal #1):

```fish
layermind monitor
```

This shows everything the daemon has collected:
```
LayerMind Runtime Status
  Runtime started: 2026-...
  AI provider: OpenRouter (deepseek/deepseek-chat)

Printer: default
  Status: IDLE
  Health:
    Total prints: 42
    Failed prints: 3
    Success rate: 0.93

  Recent History:
    Last hardware change: 2026-...
```

If no data shows yet, wait a minute — the daemon needs time to collect events.

---

## Step 10: Run an AI Diagnostic

With the daemon still running:

```fish
layermind diagnose
```

**What happens:**
1. LayerMind fetches the printer context (health, history, issues)
2. Builds a structured prompt describing the printer's state
3. Sends it to OpenRouter (or whatever provider you set)
4. Parses the AI response
5. Validates claims against known facts
6. Prints actionable recommendations

**Output:**
```
AI Diagnostic
  Summary: PID tuning recommended to improve temperature stability
  Confidence: 0.85

  Actions:
    1. Run PID calibration on extruder
       Command: PID_CALIBRATE HEATER=extruder TARGET=210
    2. Verify silicone sock is properly seated
```

---

## Step 11: Launch the TUI

Close the daemon (Ctrl+C) or open a third terminal — the TUI connects directly to Moonraker, it doesn't need the daemon:

```fish
layermind-tui
```

**What you see:**

```
 LAYERMIND ◆ voron-0 ◆ IDLE ◆ 00:00:00 ◆ Layer 0/0
───────────────────────────────────────────────────────────
 STATE                  TEMPERATURES
─────────────────────  ───────────────────────────────────
 Host:     voron-0     Extruder   235°C / 240°C
 Status:   IDLE        ▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░
 Print:    benchy      Bed        105°C / 110°C
 Progress: 47.3%       ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓
 Position: X125.0      ───────────────────────────────────
           Y150.0      PROGRESS
           Z200.0      ───────────────────────────────────
 Speed:    80 mm/s     ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░
                       47.3%   Layer 142/300  ETA 00:42:15
───────────────────────────────────────────────────────────
 EVENTS                 DIAGNOSTICS
─────────────────────  ───────────────────────────────────
 ● Connected to MR      Press d to run AI diagnostic
 ● Print started:
   benchy.gcode
───────────────────────────────────────────────────────────
 q:quit  d:diagnose  m:machine info  TAB:cycle focus
```

**Keyboard controls:**

| Key | Action |
|---|---|
| `q` | Quit the TUI |
| `d` | Run AI diagnostic (result appears in DIAGNOSTICS panel) |
| `m` | Open machine info popup (hardware profile overlay) |
| `M` | Close machine info popup |
| `Tab` | Cycle focus between panels |
| `↑` | Scroll events panel up |
| `↓` | Scroll events panel down |

**What updates automatically (every 2 seconds):**
- Temperatures (extruder + bed)
- Print progress and layer count
- Toolhead position and speed
- Printer state changes (idle → printing → complete)
- Events (print started, completed, failed, warnings)

---

## Step 12: What Happens During a Print

Start a print on your printer (any benchy, calibration cube, etc.). Here's what LayerMind does:

**Immediately:**
- Moonraker detects state change → `printing`
- TUI shows "Print started: benchy.gcode" in events panel
- Progress bar starts moving
- Temperatures are tracked in real time

**Every 2 seconds:**
- Temperature, position, and speed update in the TUI
- Events panel stays at the bottom, auto-scrolling

**When the print finishes:**
- Events panel shows "Print completed"
- Progress bar hits 100%
- Timeline records the successful print

**If the print fails:**
- Events panel shows "Print failed: thermal runaway" in red
- Timeline records the failure with the reason
- ContextStore increments failure count
- Learning engine notes the failure pattern

**Run `layermind diagnose` after a failure** — the AI will have real context to analyze.

---

## Step 13: Run the Tests

```fish
cd ~/dev/LayerMind
cargo test --workspace
```

Expected: **241 passed, 0 failed.**

Run specific tests:
```fish
cargo test -p layermind-machine   # Hardware discovery tests
cargo test -p layermind-history   # Timeline tests
cargo test -p layermind-core      # Pipeline tests
```

---

## Step 14: Update LayerMind

```fish
cd ~/dev/LayerMind
git pull                    # Get latest code
cargo build --release       # Rebuild
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/
```

---

## Quick Reference Card

```fish
# ── ONE-TIME SETUP ──────────────────────────────────────────
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone git@github.com:webbwerkx/LayerMind.git ~/dev/LayerMind
cd ~/dev/LayerMind && cargo build --release
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/

echo 'set -x MOONRAKER_URL ws://voron-0.local:7125/websocket' >> ~/.config/fish/config.fish
echo 'set -x LAYERMIND_PROVIDER openrouter' >> ~/.config/fish/config.fish
echo 'set -x LAYERMIND_MODEL deepseek/deepseek-chat' >> ~/.config/fish/config.fish
source ~/.config/fish/config.fish

# ── EVERY SESSION ───────────────────────────────────────────
layermind printer test     # Test connection (no daemon needed)
layermind run              # Start daemon (leave running)
layermind monitor          # Check status (daemon needed)
layermind diagnose         # AI diagnostic (daemon + AI needed)
layermind-tui              # Real-time TUI (no daemon needed)

# ── TUI CONTROLS ────────────────────────────────────────────
# q = quit    d = diagnose    m = machine info popup
# Tab = cycle focus    ↑↓ = scroll events

# ── UPDATE ──────────────────────────────────────────────────
cd ~/dev/LayerMind && git pull && cargo build --release
cp target/release/layermind ~/.local/bin/
cp target/release/layermind-tui ~/.local/bin/
```

---

## Troubleshooting

"I get 'logging initialized' then an error about AI provider":
→ You copied the old binary. Copy the new one: `cp target/release/layermind ~/.local/bin/`

"Connection refused when running printer test":
→ Your `MOONRAKER_URL` is wrong. Verify with `curl http://voron-0.local:7125/printer/info`

"No context available" for monitor/diagnose:
→ The daemon (`layermind run`) must be running in another terminal

TUI shows "Disconnected":
→ Check your Moonraker URL. The TUI retries every 2 seconds.

AI diagnostic returns low confidence:
→ Run the daemon through a full print cycle to gather more context

"Database unavailable, using in-memory sink":
→ Normal. PostgreSQL is optional. Everything works fine without it.
