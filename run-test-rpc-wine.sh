#!/bin/bash
echo "Run cargo build with specified arguments"
cargo +nightly-2024-05-02 build --manifest-path openzt-test-rpc/Cargo.toml --lib --release --target=i686-pc-windows-gnu

echo "Check if the build succeeded"
if [ "$retval" -ne 0 ]; then
    echo "Cargo build failed."
    exit 1
fi

ZT_DIR="$HOME/.wine/drive_c/Program Files/Microsoft Games/Zoo Tycoon"
if [ -n "$1" ]; then
  ZT_DIR="$1"
fi

echo "Remove conflicting dlls"
rm "$ZT_DIR/res-openzt.dll"
rm "$ZT_DIR/res-openzt-rpc.dll"

echo "Copy the file to the destination"
cp "target/i686-pc-windows-gnu/release/openzttestrpc.dll" "$ZT_DIR/res-openzt-rpc.dll"
echo "Check copy succeeded"

echo "Run the zoo.exe executable"
wine "$ZT_DIR/zoo.exe"
