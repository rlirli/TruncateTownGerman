#!/bin/bash

# Ensure we are in the dict_builder directory
cd "$(dirname "$0")"

mkdir -p support_data
cd support_data

# Define file constants
DE_FULL_FILE="de_full.txt"
DECOW_7Z_FILE="decow_wordfreq_cistem.csv.7z"
DEWIKI_ZIP_FILE="de_wiki_wordrank.zip"
DEWIKI_TXT_FILE="de_wiki_wordrank.txt"

# Define URLs
DE_FULL_URL="https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/de/de_full.txt"
DECOW_7Z_URL="https://nlp-data-filestorage.s3.eu-central-1.amazonaws.com/word-frequencies/decow_wordfreq_cistem.csv.7z"
DEWIKI_ZIP_URL="https://github.com/gambolputty/dewiki-wordrank/raw/main/result.zip"

# Initialize summary tracking
SUMMARY_DE_FULL="Already present"
SUMMARY_DECOW_7Z="Already present"
SUMMARY_DECOW_CSV="Already present"
SUMMARY_DEWIKI_ZIP="Already present"
SUMMARY_DEWIKI_TXT="Already present"

STATUS="Success"

# Helper function for error handling
handle_error() {
    echo -e "\nERROR: $1"
    STATUS="Failed ($1)"
    print_summary
    exit 1
}

print_summary() {
    echo -e "\n============================================="
    echo "            DATA SETUP SUMMARY               "
    echo "============================================="
    echo "Status: $STATUS"
    echo ""
    printf "%-30s | %s\n" "Resource" "Status"
    echo "-------------------------------+-------------"
    printf "%-30s | %s\n" "de_full.txt" "$SUMMARY_DE_FULL"
    printf "%-30s | %s\n" "decow...cistem.csv.7z" "$SUMMARY_DECOW_7Z"
    printf "%-30s | %s\n" "decow...cistem.csv" "$SUMMARY_DECOW_CSV"
    printf "%-30s | %s\n" "de_wiki_wordrank.zip" "$SUMMARY_DEWIKI_ZIP"
    printf "%-30s | %s\n" "de_wiki_wordrank.txt" "$SUMMARY_DEWIKI_TXT"
    echo "============================================="
}

echo "Checking $DE_FULL_FILE..."
if [ ! -f "$DE_FULL_FILE" ]; then
    echo "Downloading $DE_FULL_FILE..."
    curl -L "$DE_FULL_URL" -o "$DE_FULL_FILE" || handle_error "Failed to download $DE_FULL_FILE"
    SUMMARY_DE_FULL="Fetched"
fi

echo "Checking $DECOW_7Z_FILE..."
if [ ! -f "$DECOW_7Z_FILE" ]; then
    echo "Downloading $DECOW_7Z_FILE from Amazon S3..."
    curl -L "$DECOW_7Z_URL" -o "$DECOW_7Z_FILE" || handle_error "Failed to download 7z archive"
    SUMMARY_DECOW_7Z="Fetched"
fi

echo "Extracting $DECOW_7Z_FILE..."
if command -v 7z &> /dev/null; then
    # -aos skips extraction if the file already exists, removing the need for an explicit bash if [ -f ]
    7z x "$DECOW_7Z_FILE" -aos > /dev/null || handle_error "Failed to extract 7z archive"
    SUMMARY_DECOW_CSV="Extracted/Present"
else
    handle_error "7z command not found. Please install p7zip."
fi

echo "Checking $DEWIKI_ZIP_FILE..."
if [ ! -f "$DEWIKI_ZIP_FILE" ]; then
    echo "Downloading $DEWIKI_ZIP_FILE..."
    curl -L "$DEWIKI_ZIP_URL" -o "$DEWIKI_ZIP_FILE" || handle_error "Failed to download $DEWIKI_ZIP_FILE"
    SUMMARY_DEWIKI_ZIP="Fetched"
fi

echo "Extracting $DEWIKI_ZIP_FILE..."
if [ ! -f "$DEWIKI_TXT_FILE" ]; then
    unzip -p "$DEWIKI_ZIP_FILE" result.txt > "$DEWIKI_TXT_FILE" || handle_error "Failed to extract $DEWIKI_TXT_FILE"
    SUMMARY_DEWIKI_TXT="Extracted"
else
    SUMMARY_DEWIKI_TXT="Already present"
fi

print_summary

if [ "$STATUS" = "Success" ]; then
    echo -e "\nSetup complete. You can now run 'cargo run --release' in the dict_builder directory."
fi
