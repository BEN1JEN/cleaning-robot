use gpio::sysfs::{SysFsGpioInput, SysFsGpioOutput};
use gpio::{GpioIn, GpioOut};
use std::{thread, time};

const IR_LEFT_PIN: u32 = 4;
const IR_RIGHT_PIN: u32 = 17;
const DIST_FRONT_TRIGGER_PIN: u32 = 27;
const DIST_FRONT_ECHO_PIN: u32 = 22;
const MOTOR_LEFT_EN_PIN: u32 = 14;
const MOTOR_LEFT_IN0_PIN: u32 = 15;
const MOTOR_LEFT_IN1_PIN: u32 = 18;
const MOTOR_RIGHT_EN_PIN: u32 = 10;
const MOTOR_RIGHT_IN0_PIN: u32 = 9;
const MOTOR_RIGHT_IN1_PIN: u32 = 11;

struct Motor {
	en: SysFsGpioOutput,
	in0: SysFsGpioOutput,
	in1: SysFsGpioOutput,
	speed: f32,
	freq: f32,
	on_time: f32,
	on: bool,
}

impl Motor {
	fn new(en_pin: u32, in0_pin: u32, in1_pin: u32) -> Motor {
		Motor {
			en: SysFsGpioOutput::open(en_pin).unwrap(),
			in0: SysFsGpioOutput::open(in0_pin).unwrap(),
			in1: SysFsGpioOutput::open(in1_pin).unwrap(),
			speed: 0.0,
			freq: 1000.0,
			timer: 0.0,
			on: false,
		}
	}
	fn set_speed(&mut self, speed: Option<f32>) {
		if let Some(speed) = speed {
			let speed = speed.clamp(-1.0, 1.0);
			let abs_speed = speed.abs();
			self.en.set_high();
			if abs_speed <= 0.0001 {
				self.in0.set_low();
				self.in1.set_low();
			} else if speed > 0 {
				self.in0.set_low();
				self.in1.set_high();
			} else {
				self.in0.set_high();
				self.in1.set_low();
			}
		}
	}
	fn update_pwm(&mut self, delta_time: f32) {
		self.timer += delta_time;
		if self.on {
			if self.timer >= self.speed / self.freq {
				self.on = false;
				self.en.set_low();
				self.timer = 0.0;
			}
		} else {
			if self.timer >= (1.0 - self.speed) / self.freq {
				self.on = true;
				self.en.set_high();
				self.timer = 0.0;
			}
		}
	}
}

struct Drive {
	left: Motor,
	right: Motor,
}

impl Drive {
	fn new(left_en_pin: u32, left_in0_pin: u32, left_in1_pin: u32) -> Drive {
		Drive {
			left: Motor::new(left_en_pin: u32, left_in0_pin: u32, left_in1_pin: u32),
			right: Motor::new(right_en_pin: u32, right_in0_pin: u32, right_in1_pin: u32),
		}
	}
	fn set_drive(&mut self, speed: f32, turn: f32) {
		self.left.set_speed(speed + turn);
		self.right.set_speed(speed - turn);
	}
	fn update(&mut self, delta_time: f32) {
		self.left.update_pwm(delta_time);
		self.right.update_pwm(delta_time);
	}
}

fn main() {
	let mut drive = Drive::new(
		MOTOR_LEFT_EN_PIN,
		MOTOR_LEFT_IN0_PIN,
		MOTOR_LEFT_IN1_PIN,
		MOTOR_LEFT_EN_PIN,
		MOTOR_LEFT_IN0_PIN,
		MOTOR_LEFT_IN1_PIN,
	);
	let mut last_time = Instant::now();

	drive.set_drive(0.0, 1.0);
	loop {
		let time = Instant::now();
		let delta_time = time.duration_since(last_time).as_secs();
		last_time = time;

		drive.update(delta_time);
	}
}
