use super::color::auto_pause_color;
use super::config::{PauseColor, ShimmerConfig, ShimmerMode};
use super::render::{StatusDisplay, render_frame};
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::io::stdout;
use std::time::Instant;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::time::{Duration, MissedTickBehavior, interval};

enum Message {
    Text(String),
    StatusActive(String),
    StatusDone(String),
    StatusClear,
    Pause,
    Resume,
}

struct StatusInfo {
    text: String,
    done: bool,
}

struct ShimmerMotion {
    position: usize,
    forward: bool,
    pause_until: Option<Instant>,
    last_update: Instant,
}

impl ShimmerMotion {
    fn new(now: Instant) -> Self {
        Self {
            position: 0,
            forward: true,
            pause_until: None,
            last_update: now,
        }
    }

    fn reset(&mut self, now: Instant) {
        self.position = 0;
        self.forward = true;
        self.pause_until = None;
        self.last_update = now;
    }

    fn tick(&mut self, now: Instant, text_len: usize, mode: &ShimmerMode) {
        if let Some(pause_until) = self.pause_until {
            if now < pause_until {
                return;
            }
            self.pause_until = None;
        }

        let step = match mode {
            ShimmerMode::OneWay { duration } => *duration,
            ShimmerMode::Bounce {
                forward_duration,
                backward_duration,
            } => {
                if self.forward {
                    *forward_duration
                } else {
                    *backward_duration
                }
            }
        };

        if now.duration_since(self.last_update) < step {
            return;
        }

        self.last_update = now;
        let len = text_len.max(1);

        match mode {
            ShimmerMode::OneWay { .. } => {
                self.position = (self.position + 1) % len;
                if self.position == 0 {
                    self.pause_until = Some(now + Duration::from_secs(2));
                }
            }
            ShimmerMode::Bounce { .. } => {
                if self.forward {
                    if self.position + 1 >= len {
                        self.forward = false;
                        self.pause_until = Some(now + Duration::from_secs(2));
                    } else {
                        self.position += 1;
                    }
                } else if self.position == 0 {
                    self.forward = true;
                    self.pause_until = Some(now + Duration::from_secs(2));
                } else {
                    self.position -= 1;
                }
            }
        }
    }

    fn is_paused(&self) -> bool {
        self.pause_until.is_some()
    }
}

struct SpinnerState {
    frame: usize,
    last_update: Instant,
}

impl SpinnerState {
    fn new(now: Instant) -> Self {
        Self {
            frame: 0,
            last_update: now,
        }
    }

    fn tick(&mut self, now: Instant, duration: Duration) {
        if now.duration_since(self.last_update) >= duration {
            self.frame = self.frame.wrapping_add(1);
            self.last_update = now;
        }
    }
}

struct PauseState {
    progress: f32,
    transitioning: bool,
    paused: bool,
    started_at: Option<Instant>,
}

impl PauseState {
    const TRANSITION_SECS: f32 = 5.0;

    fn new() -> Self {
        Self {
            progress: 0.0,
            transitioning: false,
            paused: false,
            started_at: None,
        }
    }

    fn begin(&mut self, now: Instant) {
        if self.transitioning || self.paused {
            return;
        }
        self.transitioning = true;
        self.started_at = Some(now);
    }

    fn resume(&mut self) {
        self.progress = 0.0;
        self.transitioning = false;
        self.paused = false;
        self.started_at = None;
    }

    fn tick(&mut self, now: Instant) {
        if !self.transitioning {
            return;
        }

        let Some(started_at) = self.started_at else {
            return;
        };

        let progress = now.duration_since(started_at).as_secs_f32() / Self::TRANSITION_SECS;
        if progress >= 1.0 {
            self.progress = 1.0;
            self.transitioning = false;
            self.paused = true;
        } else {
            self.progress = progress;
        }
    }

    fn blocks_animation(&self) -> bool {
        self.transitioning || self.paused
    }
}

/// ターミナルにシマーアニメーションを表示するハンドル。
///
/// [`Shimmer::new`] または [`Shimmer::with_config`] で生成します。
/// 内部では tokio タスクを起動し、約 60fps でレンダリングします。
///
/// ハンドルを drop すると内部タスクは abort されます。完了メッセージを
/// 表示したい場合は [`Shimmer::stop`] を呼んでください。
pub struct Shimmer {
    tx: Option<UnboundedSender<Message>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl Shimmer {
    /// デフォルト設定でシマーを開始します。
    #[must_use]
    pub fn new(initial_text: &str) -> Self {
        Self::with_config(initial_text, ShimmerConfig::default())
    }

    /// 指定した設定でシマーを開始します。
    ///
    /// この関数は tokio ランタイム上で呼び出す必要があります。
    #[must_use]
    pub fn with_config(initial_text: &str, config: ShimmerConfig) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = tx.send(Message::Text(initial_text.to_string()));

        let pause_color = match config.pause_color {
            PauseColor::Auto => auto_pause_color(config.base_color),
            PauseColor::Custom(color) => color,
        };
        let spinner_color = config.spinner_color.unwrap_or(config.base_color);
        let initial = initial_text.to_string();

        let handle = tokio::spawn(async move {
            run_loop(rx, initial, pause_color, spinner_color, config).await;
        });

        Self {
            tx: Some(tx),
            handle: Some(handle),
        }
    }

    /// 表示するテキストを差し替えます。シマー位置は左端にリセットされます。
    pub fn update(&self, text: &str) {
        self.send(Message::Text(text.to_string()));
    }

    /// ステータスを点滅表示します。
    pub fn set_status(&self, status: &str) {
        self.send(Message::StatusActive(status.to_string()));
    }

    /// ステータスを静的グレーで表示します。
    pub fn complete_status(&self, status: &str) {
        self.send(Message::StatusDone(status.to_string()));
    }

    /// ステータス表示を消去します。
    pub fn clear_status(&self) {
        self.send(Message::StatusClear);
    }

    /// シマーアニメーションを一時停止します。
    pub fn pause(&self) {
        self.send(Message::Pause);
    }

    /// 一時停止中のシマーアニメーションを再開します。
    pub fn resume(&self) {
        self.send(Message::Resume);
    }

    /// シマーを停止し、完了メッセージを stderr に出力します。
    pub async fn stop(mut self, message: &str) {
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
        eprintln!("✓ {message}");
    }

    fn send(&self, message: Message) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(message);
        }
    }
}

impl Drop for Shimmer {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

async fn run_loop(
    mut rx: UnboundedReceiver<Message>,
    initial: String,
    pause_color: crossterm::style::Color,
    spinner_color: crossterm::style::Color,
    config: ShimmerConfig,
) {
    let mut stdout = stdout();
    let _ = execute!(stdout, cursor::Hide);

    let start_time = Instant::now();
    let mut text = initial;
    let mut status: Option<StatusInfo> = None;
    let mut motion = ShimmerMotion::new(start_time);
    let mut spinner = SpinnerState::new(start_time);
    let mut pause_state = PauseState::new();

    let mut ticker = interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let now = Instant::now();
                pause_state.tick(now);

                if !pause_state.blocks_animation() {
                    motion.tick(now, text.chars().count(), &config.shimmer_mode);
                }
                spinner.tick(now, config.spinner_duration);

                let status_display = status.as_ref().map(|status| StatusDisplay {
                    text: &status.text,
                    done: status.done,
                });

                let _ = render_frame(
                    &text,
                    status_display,
                    &config.metrics,
                    motion.position,
                    spinner.frame,
                    start_time.elapsed(),
                    motion.is_paused(),
                    pause_state.progress,
                    pause_color,
                    spinner_color,
                    &config,
                );
            }

            message = rx.recv() => {
                match message {
                    Some(Message::Text(next_text)) => {
                        text = next_text;
                        motion.reset(Instant::now());
                    }
                    Some(Message::StatusActive(next_status)) => {
                        status = Some(StatusInfo {
                            text: next_status,
                            done: false,
                        });
                    }
                    Some(Message::StatusDone(next_status)) => {
                        status = Some(StatusInfo {
                            text: next_status,
                            done: true,
                        });
                    }
                    Some(Message::StatusClear) => status = None,
                    Some(Message::Pause) => pause_state.begin(Instant::now()),
                    Some(Message::Resume) => pause_state.resume(),
                    None => break,
                }
            }
        }
    }

    let _ = execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        cursor::Show
    );
}
