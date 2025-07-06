echo "Run cargo build with specified arguments"
cargo +nightly-2024-05-02-i686-pc-windows-msvc build --lib --release --target=i686-pc-windows-msvc %*

echo "Check if the build succeeded"
if [ $retval -ne 0 ]; then
    echo "Cargo build failed."
    exit 1
fi

echo "Copy the file to the destination"
copy "target\i686-pc-windows-msvc\release\openzt.dll" "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\lang301-openzt.dll"
echo "Check copy succeeded"

echo "Run the zoo.exe executable"
wine "C:\Program Files (x86)\Microsoft Games\Zoo Tycoon\zoo.exe"
