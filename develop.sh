#!/bin/bash

mkdir -p develop
cd develop

# Download BOSS TONE STUDIO for Katana Gen 3, Windows release from Roland
BOSS_TONE_STUDIO_ZIP_PATH=ktn3_bts_w110.zip
BOSS_TONE_STUDIO_ZIP_URL=https://static.roland.com/assets/media/zip/$BOSS_TONE_STUDIO_ZIP_PATH
wget $BOSS_TONE_STUDIO_ZIP_URL -O $BOSS_TONE_STUDIO_ZIP_PATH