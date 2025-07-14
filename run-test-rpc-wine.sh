echo "Run cargo build with specified arguments"
cargo +nightly build --lib --release --target=i686-pc-windows-gnu

echo "Check if the build succeeded"
if [ $retval -ne 0 ]; then
    echo "Cargo build failed."
    exit 1
fi

echo "Remove conflicting dlls"
rm "$1/res-openzt.dll"

echo "Copy the file to the destination"
cp "target/i686-pc-windows-gnu/release/openzttestrpc.dll" "$1/res-openzt-rpc.dll"
echo "Check copy succeeded"

echo "Run the zoo.exe executable"
wine "$1/zoo.exe"
