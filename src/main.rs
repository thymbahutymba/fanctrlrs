use failure::Error;
use rust_gpiozero::output_devices;
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};

macro_rules! CFG_FILE {
    () => { "../Config.toml"; };
}

#[derive(Deserialize, Debug)]
struct Config {
    pin: u8,
    seconds: u64,
    temperature: Temperature,
    telegram: Option<TelegramConf>,
}

#[derive(Deserialize, Debug)]
struct Temperature {
    file: String,
    max: u64,
    min: u64,
}

#[derive(Deserialize, Debug)]
struct TelegramConf {
    token: String,
    chat_id: String,
}

impl Config {
    fn load() -> Result<Self, Error> {
        Ok(toml::from_str(include_str!(CFG_FILE!()))?)
    }
}

impl Temperature {
    fn switch_condition(&self, pin: &output_devices::DigitalOutputDevice, t: u64) -> bool {
        t < self.min && pin.is_active() || t > self.max && !pin.is_active()
    }
}

#[cfg(feature = "notify")]
impl TelegramConf {
    const BASE_URL: &str = "https://api.telegram.org/bot";

    fn send_message(&self, msg: &str) -> Result<(), Error> {
        let params = [("chat_id", &(*self.chat_id)), ("text", msg)];
        let res = reqwest::Client::new()
            .post(format!("{}{}/sendMessage", BASE_URL, self.token).as_str())
            .form(&params)
            .send();

        if let Err(err) = res {
            println!("{}", err);
        }
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let config = Config::load()?;
    let mut fan_pin = output_devices::DigitalOutputDevice::new(config.pin);

    // Create atomic bool for handling interruption
    let shutdown = Arc::new(AtomicBool::new(false));

    // Register interruption with the bolean
    signal_hook::flag::register(signal_hook::SIGHUP, Arc::clone(&shutdown))?;
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(&shutdown))?;
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(&shutdown))?;

    #[cfg(feature = "notify")]
    {
        // Send message of strarting execution
        config
            .telegram
            .as_ref()
            .map_or(Ok(()), |ref t| t.send_message("fanctrlrs started."))?;
    }

    while !shutdown.load(Ordering::Relaxed) {
        let mut file = File::open(&config.temperature.file)?;
        let mut contents = String::new();

        file.read_to_string(&mut contents)?;

        let temperature = (contents.trim().parse::<u64>()?) / 1000;

        if config.temperature.switch_condition(&fan_pin, temperature) {
            fan_pin.toggle();
        }

        thread::sleep(time::Duration::from_secs(config.seconds));
    }

    #[cfg(feature = "notify")]
    {
        config
            .telegram
            .as_ref()
            .map_or(Ok(()), |ref t| t.send_message("fanctrlrs stopped."))?;
    }
    Ok(())
}
