#!/bin/bash

# Ensure we are in the dict_builder directory
cd "$(dirname "$0")"

mkdir -p support_data

echo "Downloading German word frequency list (de_full.txt)..."
curl -L https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/de/de_full.txt -o support_data/de_full.txt

echo "Setup complete. You can now run cargo run --release"
