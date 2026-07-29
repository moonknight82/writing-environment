# Writing Environment Raspberry Pi 4 image

This is a complete 64-bit Raspberry Pi OS Trixie image for a Raspberry Pi 4.
Flashing it erases the selected microSD card.

1. Keep `writing-environment-pi4.img.xz` and `SHA256SUMS` together.
2. Verify the download:
   - macOS: `shasum -a 256 -c SHA256SUMS`
   - Linux: `sha256sum -c SHA256SUMS`
3. In Raspberry Pi Imager, choose **Use custom**, select
   `writing-environment-pi4.img.xz`, select the microSD card, and write it.
4. Skip Imager's OS customisation dialog so it does not replace the appliance
   session defaults.
5. Insert the card in the Pi and boot. The first boot expands the root
   filesystem and may take a little longer than subsequent boots.

The recovery account is `writer` and this development image initially uses
`writer` as its password. Change it immediately by opening the system drawer
with **Super+Space**, choosing **Password**, or running:

```sh
writing-environment-change-password
```

Writing Environment opens full screen. **Super+Space** opens the system drawer,
and **Ctrl+Super+S** opens Raspberry Pi Control Centre. The **Updates** drawer
action upgrades both Raspberry Pi OS and Writing Environment from their signed
APT repositories.
