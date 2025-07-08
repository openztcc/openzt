echo "Run cargo build with specified arguments"
cargo build --lib --release --target=i686-pc-windows-msvc %*

echo "Check if the build succeeded"
if [ $retval -ne 0 ]; then
    echo "Cargo build failed."
    exit 1
fi

echo "Copy the file to the destination"
cp "target/i686-pc-windows-msvc/release/openzt.dll" "$1/res-openzt.dll"
echo "Check copy succeeded"

echo "Run the zoo.exe executable"
wine "C:/Program Files (x86)/Microsoft Games/Zoo Tycoon/zoo.exe"
