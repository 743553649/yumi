# --- Main & Monitor ---
yumi-module-starting = yumi-module Unified Starting...
scheduler-module-started = Scheduler module started.
scheduler-module-start-failed = Failed to start scheduler module: { $error }
monitor-module-crashed = Monitor module crashed: { $error }
monitor-module-started = Monitor module started.
monitor-starting = Starting yumi-monitor module...
monitor-initial-config-failed = [Main] Failed to read initial config: { $error }.
    Using default.
monitor-screen-watcher-failed = [Main] Screen state watcher thread crashed: { $error }
monitor-config-watcher-failed = [Main] Config watcher thread crashed: { $error }
monitor-fps-crashed = [Main] FPS Monitor crashed: { $error }
monitor-fps-tokio-failed = [Main] Failed to create Tokio runtime for FPS monitor
monitor-cpu-crashed = [Main] CPU Load Monitor crashed: { $error }
monitor-cpu-tokio-failed = [Main] Failed to create Tokio runtime for CPU monitor
monitor-rlimit-memlock-failed = [Main] Failed to raise RLIMIT_MEMLOCK. eBPF maps might fail to load.

# --- AppDetect ---
app-detect-config-watch = [AppDetect] Started watching config file: { $path }
app-detect-change-detected = [AppDetect] Change detected, debouncing (100ms)...
app-detect-reloading = [AppDetect] Debounce finished. Reloading config...
app-detect-load-failed = [AppDetect] Failed: { $error }. Using default.
app-detect-reload-success = [AppDetect] Config reloaded successfully.
app-detect-loop-started = [AppDetect] App detection loop started (3000ms poll).
app-detect-screen-changed = [AppDetect] Screen state changed: { $old } -> { $new }
app-detect-mode-change-pkg = [AppDetect] Mode change: { $old } -> { $new } ({ $pkg })
app-detect-ime-auto = [AppDetect] Auto-detected IME: { $pkg }
app-detect-ime-fallback = [AppDetect] Failed to auto-detect IME, using fallback list.

# --- ScreenDetect ---
screen-state-change-detected = [Screen] State change detected via '{ $source }'.
screen-state-changed-value = [Screen] Screen state changed: { $state }
screen-netlink-started = [Screen] Started netlink-sys socket listener.

# --- Monitors ---
cpu-monitor-started = [CPU Monitor] eBPF System Load monitor started (Long-task blind spot fixed).
cpu-monitor-online-cpus-failed = [CPU Monitor] Failed to get online CPUs: { $error }
cpu-monitor-online-cpus = [CPU Monitor] Detected online CPU core IDs: { $cpus }
cpu-monitor-fg-pid-updated = [CPU Monitor] Foreground PID updated { $old } -> { $new }
cpu-monitor-tick-log = [CPU Monitor] cores=[{ $cores }] fg_pid={ $pid } fg_max_util={ $util }% threads_tracked={ $threads } delta={ $delta }ms
cpu-monitor-channel-closed = [CPU Monitor] Channel closed, exiting loop.
fps-monitor-init = [FPS Monitor] Initializing eBPF FPS monitor...
fps-monitor-attached = [FPS Monitor] Attached uprobe to PID: { $pid }
fps-monitor-attach-failed = [FPS Monitor] Failed to attach any Uprobe symbols!
fps-monitor-attach-failed-initial = [FPS Monitor] Initial attach failed: { $error }
fps-monitor-init-no-pid = [FPS Monitor] No foreground PID yet, waiting...
fps-monitor-pid-filter-updated = [FPS Monitor] Target PID updated: { $old } -> { $new }
fps-monitor-pid-switching = [FPS Monitor] Switching target PID: { $pid }
fps-monitor-pid-switched = [FPS Monitor] Switched to target PID: { $pid }
fps-monitor-pid-switch-failed = [FPS Monitor] PID switch failed: { $error }
fps-monitor-started = [FPS Monitor] eBPF FPS monitor started (per-PID uprobe mode)

# --- Scheduler ---
scheduler-ipc-started = [Scheduler] IPC Channel listener started.
scheduler-mode-change-request = [Scheduler] Mode change request: { $old } -> { $new } (Pkg: { $pkg }, Temp: { $temp })
scheduler-apply-failed = [Scheduler] Failed to apply settings: { $error }
scheduler-channel-closed = [Scheduler] Channel closed! Thread exiting.
scheduler-doze-enable = [Scheduler] Screen OFF: Enabling Extreme Doze mode (Restricting CPU max performance).
scheduler-doze-restore = [Scheduler] Screen ON: Restoring previous performance constraints.
scheduler-clg-init = [Scheduler] CPU Load Governor: initialized at startup (mode={ $mode })

# --- Scheduler: Config Watcher ---
config-reloading = [Config] Config file change detected, reloading...
config-reloaded-success = [Config] Config reloaded successfully.
config-reload-fail = [Config] Config reload failed: { $error }
config-watch-error = [Config] Failed to watch config directory: { $error }
config-apply-mode-failed = [Config] Failed to apply reloaded mode settings: { $error }
config-apply-tweaks-failed = [Config] Failed to apply reloaded system tweaks: { $error }

# --- SysFS (shared FastWriter) ---
sysfs-open-failed = [SysFS] Failed to open { $path }: { $error }
sysfs-umount2-failed = [SysFS] umount2({ $path }) failed: { $error }
sysfs-write-freq-failed = [SysFS] Write freq { $freq } failed: { $error }
sysfs-write-text-failed = [SysFS] Write text { $value } failed: { $error }

# --- CPUSet ---
cpuset-init = [CPUSet] init done | root={ $path } | groups={ $count }
cpuset-init-failed = [CPUSet] init failed: { $error }
cpuset-no-root = [CPUSet] cpuset mount point not found, CPUSet management unavailable
cpuset-no-groups = [CPUSet] no usable cpuset groups found, CPUSet management unavailable
cpuset-not-initialized = [CPUSet] not initialized, skipping mode apply
cpuset-applied = [CPUSet] applied { $mode } mode: { $detail }
cpuset-apply-failed = [CPUSet] failed to apply mode: { $error }
cpuset-partial-failed = [CPUSet] { $mode } mode: { $failed } group(s) failed to write

# --- IdleDive (CPU Idle Dive) ---
idle-dive-init = [IdleDive] CPU idle dive controller initialized
idle-dive-init-failed = [IdleDive] init failed: { $error }
idle-dive-unavailable = [IdleDive] cpuidle node unavailable, CPU idle dive disabled
idle-dive-enter = [IdleDive] entering dive state
idle-dive-exit = [IdleDive] exiting dive state
idle-dive-enter-dozed = [IdleDive] entering doze dive state
idle-dive-exit-dozed = [IdleDive] exiting doze dive state

# --- TouchBoost (Touch Boost) ---
touch-boost-init = [TouchBoost] touch boost controller initialized
touch-boost-init-failed = [TouchBoost] init failed: { $error }
touch-boost-no-device = [TouchBoost] no touch input device found, TouchBoost disabled
touch-boost-epoll-failed = [TouchBoost] epoll creation failed
touch-boost-listener-started = [TouchBoost] listener started, watching { $count } device(s)
touch-boost-thread-started = [TouchBoost] thread started
touch-boost-start = [TouchBoost] touch start, applying boost
touch-boost-release = [TouchBoost] released, starting decay recovery
touch-boost-recovered = [TouchBoost] recovery complete
touch-boost-reapply = [TouchBoost] re-touch during recovery, reapplying boost

# --- CLG ---
clg-init = [CLG] P{ $pid } init | cores={ $cpus } | freqs={ $fmin }-{ $fmax } MHz | P={ $perf } -> { $freq } kHz
clg-activated = [CLG] CPU Load Governor activated, taking over { $count } cluster(s)
clg-no-clusters = [CLG] CPU Load Governor: no valid clusters found, staying inactive
clg-deactivated = [CLG] CPU Load Governor deactivated
clg-config-reloaded = [CLG] config hot-reloaded | up={ $up } down={ $down } floor={ $floor } ceil={ $ceil }
clg-tick-log = [CLG] P{ $pid } util={ $util }% perf={ $perf } freq={ $freq }kHz{ $boost }
clg-writer-invalid = [CLG] P{ $pid } sysfs writer invalid (max_valid: { $max_valid }, min_valid: { $min_valid }), skipping.

# --- FAS ---
fas-freq-mismatch = [FAS] P{ $pid }: freq mismatch! expected { $min }-{ $max }, actual { $actual } -> emergency reapply
fas-auto-capacity = [FAS] auto capacity weight:
fas-auto-capacity-core = [FAS]   P{ $pid }: cap={ $cap } -> w={ $weight }
fas-policy-init = [FAS] P{ $pid } { $min }-{ $max } MHz | w={ $weight }
fas-init-summary = [FAS] init | { $fps }fps margin:{ $margin } clusters:{ $clusters } P:{ $perf } profiles:{ $profiles }
fas-app-switch = [FAS] app switch ({ $ms }ms) | P -> { $perf }
fas-loading-start = [FAS] entering loading state ({ $frames } frames, { $ms }ms) | P { $old_perf } -> { $new_perf }
fas-loading-exit = [FAS] exit loading state | P -> { $perf }
fas-gear-switch = [FAS] gear switch { $old } -> { $new }fps | P -> { $perf }
fas-low-perf-upgrade = [FAS] low-load steady frame upgrade | P={ $perf } avg={ $avg } stddev={ $stddev } -> { $fps }fps
fas-downgrade-boost = [FAS] downgrade boost | avg:{ $avg } | P { $old } -> { $new } (inc={ $inc })
fas-boost-expired = [FAS] boost expired, fast-tracking downgrade (confirm={ $confirm })
fas-floor-rescue = [FAS] floor-rescue | stuck { $frames } frames at P={ $old }, avg:{ $avg } -> P:{ $new }
fas-tick-log = [FAS] { $target }fps avg:{ $avg } | { $ms }ms ema:{ $ema } | err:{ $err_ema }/{ $err_inst } | { $act } | P:{ $perf } fg_util:{ $util }{ $cd }{ $damp }{ $temp }{ $offset }
fas-set-game = [FAS] set_game | pkg={ $pkg } | gears={ $gears } | target={ $target }fps
fas-no-profile = [FAS] no per-app profile for '{ $pkg }', using global gears { $gears }
fas-ignore-write = [FAS] P{ $pid } ignore_write = { $ignore }
fas-pid-reloaded = [FAS] PID coefficients hot-reloaded: Kp={ $kp } Ki={ $ki } Kd={ $kd }
fas-rules-reloaded = [FAS] rules hot-reloaded (margin={ $margin }, floor={ $floor }, ceil={ $ceil }, profiles={ $profiles })
fas-policy-writer-invalid = [FAS] P{ $pid } policy writer invalid (max_valid: { $max_valid }, min_valid: { $min_valid }), skipping.

# --- Scheduler: Settings ---
apply-settings-for-mode = Applying settings for mode: { $mode }
settings-applied-success = Settings for mode '{ $mode }' applied successfully.
apply-cpu-idle-governor-start = CPU idle governor settings applied.
apply-io-settings-start = I/O settings applied.
main-config-watch-thread-create = Main config watcher thread created.
config-file-changed = [Config] File changed: { $file }
cpuset-config-reloaded = [CPUSet] Config reloaded
idle-dive-config-reloaded = [IdleDive] Config reloaded
touch-boost-config-reloaded = [TouchBoost] Config reloaded

# --- Logger ---
log-level-updated = Log level updated to: { $level }
scheduler-ipc-panicked = [Scheduler] IPC thread panicked: { $error }
touch-boost-channel-disconnected = [TouchBoost] Touch event channel disconnected, TouchBoost disabled

# --- GPU Management ---
gpu-init = [GPU] GPU manager initialized
gpu-mode-switch = [GPU] Mode switch: →{ $mode } latency={ $ms }ms freq={ $freq }Hz
gpu-enter-doze = [GPU] Entering screen-off GPU power-saving mode
gpu-exit-doze = [GPU] Exiting screen-off GPU power-saving mode
gpu-release = [GPU] Released GPU control, restored defaults
gpu-unavailable = [GPU] kgsl sysfs unavailable, GPU control disabled
gpu-insufficient-freqs = [GPU] Insufficient frequencies ({ $count }), GPU control disabled
gpu-init-failed = [GPU] Initialization failed: { $error }
gpu-write-failed = [GPU] Write to { $node } failed: { $error }
gpu-circuit-breaker = [GPU] Write circuit breaker tripped, cooling { $secs }s
gpu-watchdog-detected = [GPU] Watchdog detected GPU frequency stall
gpu-watchdog-recovered = [GPU] Watchdog recovered successfully
gpu-watchdog-hung = [GPU] GPU unresponsive, relinquishing control
gpu-keepalive-started = [GPU] Keepalive thread started (interval { $secs }s)
gpu-config-reloaded = [GPU] Config hot-reloaded
