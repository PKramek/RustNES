use std::collections::VecDeque;

const CPU_CLOCK_HZ: u64 = 1_789_773;
const DEFAULT_SAMPLE_RATE: u32 = 44_100;
const MAX_BUFFERED_SAMPLES: usize = 8_192;
const HALF_FRAME_CYCLES: u32 = 14_915;
const FRAME_IRQ_CYCLES: u32 = HALF_FRAME_CYCLES * 2;
const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];
const DUTY_PATTERNS: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],
    [0, 1, 1, 0, 0, 0, 0, 0],
    [0, 1, 1, 1, 1, 0, 0, 0],
    [1, 0, 0, 1, 1, 1, 1, 1],
];
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];
const NOISE_PERIODS: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1_016, 2_034, 4_068,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCounterMode {
    FourStep,
    FiveStep,
}

#[derive(Debug)]
pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    frame_counter: FrameCounter,
    frame_irq_pending: bool,
    half_frame_divider: u32,
    frame_irq_divider: u32,
    sample_rate: u32,
    sample_clock: u64,
    sample_buffer: VecDeque<f32>,
}

impl Default for Apu {
    fn default() -> Self {
        Self {
            pulse1: PulseChannel::default(),
            pulse2: PulseChannel::default(),
            triangle: TriangleChannel::default(),
            noise: NoiseChannel::default(),
            frame_counter: FrameCounter::default(),
            frame_irq_pending: false,
            half_frame_divider: 0,
            frame_irq_divider: 0,
            sample_rate: DEFAULT_SAMPLE_RATE,
            sample_clock: 0,
            sample_buffer: VecDeque::with_capacity(MAX_BUFFERED_SAMPLES),
        }
    }
}

impl Apu {
    pub fn frame_counter_mode(&self) -> FrameCounterMode {
        self.frame_counter.mode
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate.max(1);
        self.sample_clock = 0;
        self.sample_buffer.clear();
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn read_status(&mut self) -> u8 {
        let status = self.channel_status_bits() | if self.frame_irq_pending { 0x40 } else { 0x00 };
        self.frame_irq_pending = false;
        status
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x4000 => self.pulse1.write_control(value),
            0x4001 => self.pulse1.write_sweep(value),
            0x4002 => self.pulse1.write_timer_low(value),
            0x4003 => self.pulse1.write_timer_high_and_length(value),
            0x4004 => self.pulse2.write_control(value),
            0x4005 => self.pulse2.write_sweep(value),
            0x4006 => self.pulse2.write_timer_low(value),
            0x4007 => self.pulse2.write_timer_high_and_length(value),
            0x4008 => self.triangle.write_linear_control(value),
            0x400A => self.triangle.write_timer_low(value),
            0x400B => self.triangle.write_timer_high_and_length(value),
            0x400C => self.noise.write_control(value),
            0x400E => self.noise.write_period(value),
            0x400F => self.noise.write_length(value),
            0x4015 => self.write_status(value),
            0x4017 => self.write_frame_counter(value),
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        self.pulse1.tick();
        self.pulse2.tick();
        self.triangle.tick();
        self.noise.tick();

        self.half_frame_divider = self.half_frame_divider.saturating_add(1);
        while self.half_frame_divider >= HALF_FRAME_CYCLES {
            self.half_frame_divider -= HALF_FRAME_CYCLES;
            self.clock_length_counters();
        }

        if matches!(self.frame_counter.mode, FrameCounterMode::FourStep)
            && !self.frame_counter.irq_inhibit
        {
            self.frame_irq_divider = self.frame_irq_divider.saturating_add(1);
            if self.frame_irq_divider >= FRAME_IRQ_CYCLES {
                self.frame_irq_divider -= FRAME_IRQ_CYCLES;
                self.frame_irq_pending = true;
            }
        }

        self.sample_clock += u64::from(self.sample_rate);
        while self.sample_clock >= CPU_CLOCK_HZ {
            self.sample_clock -= CPU_CLOCK_HZ;
            self.push_sample(self.mix_sample());
        }
    }

    pub fn take_samples(&mut self) -> Vec<f32> {
        self.sample_buffer.drain(..).collect()
    }

    fn channel_status_bits(&self) -> u8 {
        let mut status = 0;
        if self.pulse1.active() {
            status |= 0x01;
        }
        if self.pulse2.active() {
            status |= 0x02;
        }
        if self.triangle.active() {
            status |= 0x04;
        }
        if self.noise.active() {
            status |= 0x08;
        }
        status
    }

    fn write_status(&mut self, value: u8) {
        self.pulse1.set_enabled(value & 0x01 != 0);
        self.pulse2.set_enabled(value & 0x02 != 0);
        self.triangle.set_enabled(value & 0x04 != 0);
        self.noise.set_enabled(value & 0x08 != 0);
    }

    fn write_frame_counter(&mut self, value: u8) {
        self.frame_counter.mode = if value & 0x80 != 0 {
            FrameCounterMode::FiveStep
        } else {
            FrameCounterMode::FourStep
        };
        self.frame_counter.irq_inhibit = value & 0x40 != 0;
        if self.frame_counter.irq_inhibit {
            self.frame_irq_pending = false;
        }
        self.frame_irq_divider = 0;
    }

    fn clock_length_counters(&mut self) {
        self.pulse1.clock_length_counter();
        self.pulse2.clock_length_counter();
        self.triangle.clock_length_counter();
        self.noise.clock_length_counter();
    }

    fn mix_sample(&self) -> f32 {
        let pulse_mix = (self.pulse1.output() + self.pulse2.output()) * 0.35;
        let triangle_mix = self.triangle.output() * 0.40;
        let noise_mix = self.noise.output() * 0.25;
        (pulse_mix + triangle_mix + noise_mix).clamp(-1.0, 1.0)
    }

    fn push_sample(&mut self, sample: f32) {
        if self.sample_buffer.len() == MAX_BUFFERED_SAMPLES {
            self.sample_buffer.pop_front();
        }
        self.sample_buffer.push_back(sample);
    }
}

#[derive(Debug, Clone, Copy)]
struct FrameCounter {
    mode: FrameCounterMode,
    irq_inhibit: bool,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self {
            mode: FrameCounterMode::FourStep,
            irq_inhibit: false,
        }
    }
}

#[derive(Debug, Default)]
struct PulseChannel {
    enabled: bool,
    duty: usize,
    volume: u8,
    length_halt: bool,
    timer_period: u16,
    timer_counter: u16,
    sequence_step: usize,
    length_counter: u8,
}

impl PulseChannel {
    fn write_control(&mut self, value: u8) {
        self.duty = ((value >> 6) & 0x03) as usize;
        self.length_halt = value & 0x20 != 0;
        self.volume = value & 0x0F;
    }

    fn write_sweep(&mut self, _value: u8) {}

    fn write_timer_low(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x0700) | u16::from(value);
    }

    fn write_timer_high_and_length(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | (u16::from(value & 0x07) << 8);
        self.timer_counter = self.timer_reload();
        self.sequence_step = 0;
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload();
            self.sequence_step = (self.sequence_step + 1) & 0x07;
        } else {
            self.timer_counter -= 1;
        }
    }

    fn clock_length_counter(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn active(&self) -> bool {
        self.enabled && self.length_counter > 0
    }

    fn output(&self) -> f32 {
        if !self.active() || self.timer_period < 8 {
            return 0.0;
        }
        if DUTY_PATTERNS[self.duty][self.sequence_step] == 0 {
            return 0.0;
        }
        (self.volume as f32 / 15.0) * 2.0 - 1.0
    }

    fn timer_reload(&self) -> u16 {
        let reload = (u32::from(self.timer_period) + 1) * 2;
        reload.saturating_sub(1) as u16
    }
}

#[derive(Debug, Default)]
struct TriangleChannel {
    enabled: bool,
    control_flag: bool,
    linear_counter_reload: u8,
    timer_period: u16,
    timer_counter: u16,
    sequence_step: usize,
    length_counter: u8,
}

impl TriangleChannel {
    fn write_linear_control(&mut self, value: u8) {
        self.control_flag = value & 0x80 != 0;
        self.linear_counter_reload = value & 0x7F;
    }

    fn write_timer_low(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x0700) | u16::from(value);
    }

    fn write_timer_high_and_length(&mut self, value: u8) {
        self.timer_period = (self.timer_period & 0x00FF) | (u16::from(value & 0x07) << 8);
        self.timer_counter = self.timer_reload();
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload();
            self.sequence_step = (self.sequence_step + 1) & 0x1F;
        } else {
            self.timer_counter -= 1;
        }
    }

    fn clock_length_counter(&mut self) {
        if !self.control_flag && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn active(&self) -> bool {
        self.enabled && self.length_counter > 0 && self.linear_counter_reload > 0
    }

    fn output(&self) -> f32 {
        if !self.active() || self.timer_period < 2 {
            return 0.0;
        }
        (TRIANGLE_SEQUENCE[self.sequence_step] as f32 / 15.0) * 2.0 - 1.0
    }

    fn timer_reload(&self) -> u16 {
        self.timer_period.max(1)
    }
}

#[derive(Debug)]
struct NoiseChannel {
    enabled: bool,
    volume: u8,
    length_halt: bool,
    mode: bool,
    period_index: usize,
    timer_counter: u16,
    length_counter: u8,
    shift_register: u16,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0,
            length_halt: false,
            mode: false,
            period_index: 0,
            timer_counter: NOISE_PERIODS[0],
            length_counter: 0,
            shift_register: 1,
        }
    }
}

impl NoiseChannel {
    fn write_control(&mut self, value: u8) {
        self.length_halt = value & 0x20 != 0;
        self.volume = value & 0x0F;
    }

    fn write_period(&mut self, value: u8) {
        self.mode = value & 0x80 != 0;
        self.period_index = (value & 0x0F) as usize;
        self.timer_counter = self.timer_reload();
    }

    fn write_length(&mut self, value: u8) {
        if self.enabled {
            self.length_counter = LENGTH_TABLE[(value >> 3) as usize];
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_reload();
            let tap_bit = if self.mode { 6 } else { 1 };
            let feedback = (self.shift_register ^ (self.shift_register >> tap_bit)) & 0x0001;
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer_counter -= 1;
        }
    }

    fn clock_length_counter(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    fn active(&self) -> bool {
        self.enabled && self.length_counter > 0
    }

    fn output(&self) -> f32 {
        if !self.active() || (self.shift_register & 0x0001) != 0 {
            return 0.0;
        }
        (self.volume as f32 / 15.0) * 2.0 - 1.0
    }

    fn timer_reload(&self) -> u16 {
        NOISE_PERIODS[self.period_index]
    }
}
