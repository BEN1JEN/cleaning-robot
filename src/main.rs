use gpio::sysfs::{SysFsGpioInput, SysFsGpioOutput};
use gpio::{GpioIn, GpioOut, GpioValue};

use std::time::Instant;

const IR_LEFT_PIN: u16 = 4;
const IR_RIGHT_PIN: u16 = 17;
const DIST_FRONT_TRIGGER_PIN: u16 = 27;
const DIST_FRONT_ECHO_PIN: u16 = 22;
const MOTOR_LEFT_EN_PIN: u16 = 18;
const MOTOR_LEFT_IN0_PIN: u16 = 23;
const MOTOR_LEFT_IN1_PIN: u16 = 24;
const MOTOR_RIGHT_EN_PIN: u16 = 10;
const MOTOR_RIGHT_IN0_PIN: u16 = 9;
const MOTOR_RIGHT_IN1_PIN: u16 = 11;

struct Motor {
	en: SysFsGpioOutput,
	in0: SysFsGpioOutput,
	in1: SysFsGpioOutput,
	speed: f32,
	freq: f32,
	timer: f32,
	on: bool,
}

impl Motor {
	fn new(en_pin: u16, in0_pin: u16, in1_pin: u16) -> Motor {
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
			self.en.set_high().unwrap();
			if abs_speed <= 0.0001 {
				self.in0.set_low().unwrap();
				self.in1.set_low().unwrap();
			} else if speed > 0.0 {
				self.in0.set_low().unwrap();
				self.in1.set_high().unwrap();
			} else {
				self.in0.set_high().unwrap();
				self.in1.set_low().unwrap();
			}
			self.speed = abs_speed;
		}
	}
	fn update_pwm(&mut self, delta_time: f32) {
		self.timer += delta_time;
		if self.on {
			if self.timer >= self.speed / self.freq {
				self.on = false;
				self.en.set_low().unwrap();
				self.timer = 0.0;
			}
		} else if self.timer >= (1.0 - self.speed) / self.freq {
			self.on = true;
			self.en.set_high().unwrap();
			self.timer = 0.0;
		}
	}
}

struct Drive {
	left: Motor,
	right: Motor,
}

impl Drive {
	fn new(left_en_pin: u16, left_in0_pin: u16, left_in1_pin: u16, right_en_pin: u16, right_in0_pin: u16, right_in1_pin: u16) -> Drive {
		Drive {
			left: Motor::new(left_en_pin, left_in0_pin, left_in1_pin),
			right: Motor::new(right_en_pin, right_in0_pin, right_in1_pin),
		}
	}
	fn set_drive(&mut self, speed: f32, turn: f32) {
		self.left.set_speed(Some(-(speed + turn)));
		self.right.set_speed(Some(-(speed - turn)));
	}
	fn update(&mut self, delta_time: f32) {
		self.left.update_pwm(delta_time);
		self.right.update_pwm(delta_time);
	}
}

struct Dist {
	trigger: SysFsGpioOutput,
	echo: SysFsGpioInput,
}

impl Dist {
	fn new(trigger_pin: u16, echo_pin: u16) -> Dist {
		Dist {
			trigger: SysFsGpioOutput::open(trigger_pin).unwrap(),
			echo: SysFsGpioInput::open(echo_pin).unwrap(),
		}
	}
	fn get_dist(&mut self) -> Option<f32> {
		self.trigger.set_high().unwrap();
		std::thread::sleep(std::time::Duration::from_micros(10));
		self.trigger.set_low().unwrap();
		let start_time = Instant::now();
		while self.echo.read_value().unwrap() == GpioValue::Low  {
			if start_time.elapsed().as_secs_f32() > 0.005 {
				return None;
			}
		}
		let mut duration = start_time.elapsed();
		while self.echo.read_value().unwrap() == GpioValue::High  {
			duration = start_time.elapsed();
			if duration.as_secs_f32() > 0.005 {
				return None;
			}
		}
		let time = duration.as_secs_f32();
		let dist = time*1000000.0/58.0;
		if 2.0..15.0 {
			Some(dist)
		} else {
			None
		}
	}
}

struct IrSensor {
	sense: SysFsGpioInput,
	timer: f32,
}

impl IrSensor {
	fn new(pin: u16) -> IrSensor {
		IrSensor {
			sense: SysFsGpioInput::open(pin).unwrap(),
			timer: 0.0,
		}
	}
	fn update(&mut self, delta_time: f32) {
		self.timer += delta_time;
		if self.sense.read_value().unwrap() == GpioValue::High {
			self.timer = 0.0;
		}
	}
	fn sensing(&self) -> bool {
		self.timer < 0.2
	}
}

enum DriveState {
	Off,
	Wonder,
	TurnLeft(f32),
	TurnRight(f32),
}

fn main() {
	let mut drive = Drive::new(
		MOTOR_LEFT_EN_PIN,
		MOTOR_LEFT_IN0_PIN,
		MOTOR_LEFT_IN1_PIN,
		MOTOR_RIGHT_EN_PIN,
		MOTOR_RIGHT_IN0_PIN,
		MOTOR_RIGHT_IN1_PIN,
	);
	let mut front_distance = Dist::new(DIST_FRONT_TRIGGER_PIN, DIST_FRONT_ECHO_PIN);
	let mut left_ground = IrSensor::new(IR_LEFT_PIN);
	let mut right_ground = IrSensor::new(IR_RIGHT_PIN);
	let mut last_time = Instant::now();

	let mut state = DriveState::Wonder;

	loop {
		let time = Instant::now();
		let delta_time = time.duration_since(last_time).as_secs_f32();
		last_time = time;

		left_ground.update(delta_time);
		right_ground.update(delta_time);
		drive.update(delta_time);

		let dist = front_distance.get_dist();

		match state {
			DriveState::Wonder => {
				drive.set_drive(0.4, 0.0);
				if let Some(dist) = dist {
					if dist < 6.0 {
						state = if rand::random() {
							DriveState::TurnLeft
						} else {
							DriveState::TurnRight
						}
					}
				}
			},
			DriveState::TurnLeft(mut t) => {
				drive.set_drive(-0.4, -0.6);
				t += delta_time;
				if t >= 1.0 {
					state = DriveState::Wonder;
				}
			}
			DriveState::TurnRight(mut t) => {
				drive.set_drive(-0.4, 0.6);
				if t >= 1.0 {
					state = DriveState::Wonder;
				}
			}
			DriveState::Off => {
				drive.set_drive(-0.4, 0.6);
			}
		}

		println!("Dist: {:?}", dist);
		println!("Left: {}", left_ground.sensing());
		println!("Right: {}", right_ground.sensing());
	}
}
