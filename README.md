# ModBreeze
[![rust badge](https://img.shields.io/static/v1?label=Made%20with&message=Rust&style=for-the-badge&logo=rust&labelColor=e82833&color=b11522)](https://www.rust-lang.org/)
[![license badge](https://img.shields.io/github/license/Mr1cecream/ModBreeze?style=for-the-badge)](https://github.com/Mr1cecream/ModBreeze/blob/main/LICENSE)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/mr_icecream)

Modbreeze is a fast and easy to use mod manager for Minecraft written in Rust that allows easy sharing of modpacks with your friends using TOML.

## Installation

### Windows (via Scoop)
- Install [Scoop](https://scoop.sh/) if you don't have it already.
- Run `scoop install https://raw.githubusercontent.com/Mr1cecream/ModBreeze/main/scoop/modbreeze.json`

### Manually
- Download the zip for your platform from [the releases](https://github.com/Mr1cecream/Modbreeze/releases).
- Extract the zip into a folder of your choice.
- Optionally add the folder to PATH.

### From Source
Compiling Modbreeze requires a CurseForge API key.
If you have one of those, set the environment variable `CF_API_KEY` to it, clone the repo and run `cargo b -r`.

## Usage
Run `modbreeze -h` or `modbreeze help` for help.

### Modpack definitions
Modpacks are defined in a .TOML file, as seen in the [`example_pack.toml`](example_pack.toml).
The file should include the name of the pack, it's version, and the mod loader and Minecraft version it is made for.

Mods are split into 3 different categories: `[mods.common]`, `[mods.client]` and `[mods.server]`.
Use the common category for mods that should be installed on both the client and the server,
and the client and server categories for mods that should be installed on the client side and the server side, respectively.

Mods can be defined by simple ProjectIDs such as `mod = 123`
or with optional parameters to ignore the mod loader or Minecraft version like so:
`mod = { id = 123, ignore_loader = true, ignore_version = true }`

You can also add Resourcepacks and Shaderpacks to your packs,
the same way you would add mods, under the `[resourcepacks]` and `[shaderpacks]` categories, respectively.
> **Note**: Shaderpacks are currently unsupported due to no Customization support in the CurseForge API.

### CLI
You can download modpacks by using the command `modbreeze upgrade` and providing the source via either `-f <FILE>` or `-u <URL>`,
for sourcing local files and URLs, respectively.

Sources, along with other configuration options, will be saved for the next time automatically, so you can run `modbreeze upgrade` after setting all your preffered options. The source can be changed by running `modbreeze source` followed by the same source parameters you used in the upgrade command.

You can pass other parameters such as the mod side to download, which defaults to `client` using `-s <SIDE>` or the Minecraft root directory with `-d <DIR>`. These can also be changed by running `modbreeze config` with the same options.

To download Resourcepacks or Shaderpacks you must pass the `--resourcepacks` and `--shaderpacks` flags, respectively.
These are not saved, so you need to pass them every time you want to install or update the Resourcepacks or Shaderpacks.

## Contributing
Feel free to open an issue or pull request if you find any bugs or have improvements to the program.
Please describe the problem as detailed as possible, to make it easier to understand and fix.
