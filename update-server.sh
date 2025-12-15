#!/bin/bash
case $1 in
    '-r') build='release';;
    *|'-d') build='debug';;
esac

if command -v cargo
then
    echo "Building with \`cargo\`..."
    if [ "$build" = 'release' ]; then
        is_release=1
    fi
    if cargo build ${is_release:+'--release'}; then
        exit 0
    else
        echo 'Build failed.'
    fi
fi

file='gvsu-cis350-sporks.exe'
download="https://github.com/AmityWilder/gvsu-cis350-sporks/releases/latest/download/$file"
dest="$(dirname "$0")/target/$build/$file"
echo "Downloading \`$download\` into \`$dest\`..."
curl -L $download --create-dirs -o $dest
