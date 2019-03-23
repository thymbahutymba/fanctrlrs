# fanctrlrs
Fan control program written in rust with notification via telegram bot.

For make it work you have to install rust toolchain in your computer and compile
the program with `cargo`.

If you wish to have telegram notification you have to compile with notify feature. In this case your `Config.toml` have to contain the `[telegram]` section.
```
$ cargo build --feature notify --release
```
Whether you want not this feature it's enough to compile without `--feature notify`

It is also provide a systemd file service that have to be copied into the correct directory
```
# cp fanctrlrs.service /lib/systemd/system/
# systemctl start fanctrlrs
```

For automatically start on boot the systemd service the following command is required
```
# systemctl enable fanctrlrs
```
