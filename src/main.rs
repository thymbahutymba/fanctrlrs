use failure::Error;
use rust_gpiozero::output_devices;
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread, time,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Opt {
    #[structopt(long, short, help = "Path to configuration file.", parse(from_os_str))]
    config_file: PathBuf,
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
        let cfg_file = fs::canonicalize(Opt::from_args().config_file)?;
        Ok(toml::from_str(&fs::read_to_string(cfg_file)?)?)
    }
}

impl Temperature {
    fn switch_condition(&self, pin: &output_devices::DigitalOutputDevice, t: u64) -> bool {
        t < self.min && pin.is_active() || t > self.max && !pin.is_active()
    }
}

#[cfg(feature = "notify")]
impl TelegramConf {
    const BASE_URL: &'static str = "https://api.telegram.org/bot";

    fn send_message(&self, msg: &str) -> Result<(), Error> {
        let params = [("chat_id", &(*self.chat_id)), ("text", msg)];
        let res = reqwest::Client::new()
            .post(format!("{}{}/sendMessage", TelegramConf::BASE_URL, self.token).as_str())
            .form(&params)
            .send();

        if let Err(err) = res {
            println!("{}", err.source().unwrap());
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
        let contents = fs::read_to_string(&config.temperature.file)?;

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
