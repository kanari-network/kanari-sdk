# Move

Provides language support for the Move programming language. For information about Move visit the
language [documentation](https://docs..io/concepts/-move-concepts).

# How to Install

1. Open a new window in any Visual Studio Code application version 1.61.0 or greater.
2. Open the command palette (`⇧` + `⌘` + `P` on macOS, or use the menu item *View > Command Palette...*) and
   type **Extensions: Install Extensions**. This will open a panel named *Extensions* in the
   sidebar of your Visual Studio Code window.
kari3. In the search bar labeled *Search Extensions in Marketplace*, type **KanariNetwork**. The Move extension 
   should appear as one of the option in the list below the search bar. Click **Install**.
4. Open any file that ends in `.move`.

Installation of the extension will also install a platform-specific pre-built move-analyzer binary in
the default directory (see [here](#what-if-i-want-to-use-a-move-analyzer-binary-in-a-different-location)
for information on the location of this directory), overwriting the existing binary if it already exists.
The move-analyzer binary is responsible for the advanced features of this VSCode extension (e.g., go to
definition, type on hover). Please see [Troubleshooting](#troubleshooting) for situations when
the pre-built move-analyzer binary is not available for your platform or if you want to use move-analyzer
binary stored in a different location.

If you want to build and test Move code using the extension, you must install the `kari` binary on
your machine - see [here](https://docs..io/guides/developer/getting-started/-install) for
instructions. The extension assumes that the `kari` binary is in your system path, but you can set
its custom location location using VSCode's settings (`⌘` + `,` on macOS, or use the menu item *Code >
Preferences > Settings*). Search for the `move.kari.path` user setting, set it to the new location of
the `kari` binary, and restart VSCode.

# Troubleshooting

## What if the pre-built move-analyzer binary is not available for my platform?

If you are on Windows, the following answer assumes that your Windows user name is `USER`.

The `move-analyzer` language server is a Rust program which you can install manually provided
that you have Rust development already [installed](https://www.rust-lang.org/tools/install).
This can be done in two steps:

1. Install the move-analyzer installation prerequisites for your platform. They are the same
as prerequisites for Sui installation - for Linux, macOS and Windows these prerequisites and
their installation instructions can be found
[here](https://docs..io/guides/developer/getting-started/-install#additional-prerequisites-by-operating-system)
2. Invoke `cargo install --git https://github.com/kanari-network/kanari-sdk kari-move-analyzer` to install the
`kari move-analyzer` language server in your Cargo binary directory, which is typically located
in the `~/.cargo/bin` (macOS/Linux) or `C:\Users\USER\.cargo\bin` (Windows) directory.
3. Copy the move-analyzer binary to `~/./bin` (macOS/Linux) or `C:\Users\USER\.\bin`
(Windows), which is its default location (create this directory if it does not exist).

## What if I want to use a move-analyzer binary in a different location?

If you are on Windows, the following answer assumes that your Windows user name is `USER`.

If your `move-analyzer` binary is in a different directory than the default one (`~/./bin`
on macOS or Linux, or `C:\Users\USER\.\bin` on Windows), you may have the extension look
for the binary at this new location using VSCode's settings (`⌘` + `,` on macOS, or use the menu
item *Code > Preferences > Settings*). Search for the `move.server.path` user setting,
set it to the new location of the `move-analyzer` binary, and restart VSCode.

## What if advanced features (e.g., go to def) do not work, particularly after re-install or upgrade

Assuming you did not specify a different location for the move-analyzer binary and that the
move-analyzer binary already exists in the default location (`~/.kari/bin` on macOS or Linux, or
`C:\Users\USER\.kari\bin` on Windows), delete the existing move-analyzer binary and reinstall the
extension.


# Features

Here are some of the features of the Move Visual Studio Code extension. To see them, open a
Move source file (a file with a `.move` file extension) and:

- See Move keywords and types highlighted in appropriate colors.
- Comment and un-comment lines of code (`⌘` + `/` on macOS or the menu item *Edit >
  Toggle Line Comment*).
- Place your cursor on a delimiter, such as `<`, `(`, or `{`, and its corresponding delimiter --
  `>`, `)`, or `}` -- will be highlighted.
- As you type, Move keywords will appear as completion suggestions.
- If the opened Move source file is located within a buildable project (a `Move.toml` file can be
  found in one of its parent directories), the following advanced features will also be available:
  - compiler diagnostics
  - go to definition
  - go to type definition
  - go to references
  - type on hover
  - outline view showing symbol tree for Move source files
- If the opened Move source file is located within a buildable project you can build and (locally)
  test this project using `Move: Build a Move package` and `Move: Test a Move package` commands from
  VSCode's command palette
