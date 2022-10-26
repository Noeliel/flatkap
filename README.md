# Flatkap

A basic `flatpak run` wrapper that tries to kill flatpak-session-helper and flatpak-portal after you quit your flatpak app.

It can handle multiple flatpak apps running at the same time and will only attempt to kill the aforementioned processes after you quit your *last* app.  
Will not work with some apps that do weird forking magic to detach themselves from your shell like VSCode and similar.

## Usage

Build and install to your distribution's appropriate directory.  
Then, run `flatkap run <..>` as opposed to `flatpak run <..>`.

## License

This project is licensed under the LGPL-2.0-only SPDX identifier.
