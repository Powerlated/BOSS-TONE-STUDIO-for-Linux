FROM ubuntu:24.04 as

RUN apt-get update -y
RUN apt-get upgrade -y

RUN apt-get install -y build-essential git cmake ninja-build python3 libwebkit2gtk-4.1-dev curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev doxygen liblzma-dev libboost-all-dev rustup nodejs npm unzip 7zip libasound2-dev

RUN useradd -ms /bin/bash myuser
USER myuser

# Install Rust
RUN rustup default 1.88

# Download, build, and install innoextract so we can extract the BOSS TONE STUDIO installer
WORKDIR /home/myuser
ADD --chown=myuser https://github.com/dscharrer/innoextract.git#1.9 innoextract
WORKDIR /home/myuser/innoextract
RUN cmake -Bbuild -GNinja 
RUN cmake --build build
USER root
RUN cmake --install build

# Download BOSS TONE STUDIO for Katana Gen 3, Windows release from Roland
USER myuser
WORKDIR /home/myuser
ARG BOSS_TONE_STUDIO_ZIP_PATH=ktn3_bts_w110.zip
ARG BOSS_TONE_STUDIO_ZIP_URL=https://static.roland.com/assets/media/zip/$BOSS_TONE_STUDIO_ZIP_PATH
ADD --chown=myuser $BOSS_TONE_STUDIO_ZIP_URL .

# Extract the installer
RUN unzip $BOSS_TONE_STUDIO_ZIP_PATH
WORKDIR /home/myuser/installer_extracted
RUN innoextract "../BOSS TONE STUDIO for KATANA Gen 3 Installer.exe"

# Create build directory and copy WebView frontend
WORKDIR /home/myuser/build
RUN cp -rv "/home/myuser/installer_extracted/localappdata/Roland/BOSS TONE STUDIO for KATANA Gen 3/html" .

# Patch the WebView frontend to use the Tauri backend
ADD html-patches/html-katana-gen-3.patch /home/myuser/build
WORKDIR html
RUN patch -p1 -fi ../html-katana-gen-3.patch

WORKDIR /home/myuser/build
ADD --chown=myuser tauri .
WORKDIR tauri
RUN npm install
RUN npm run tauri build

# Copy built packages into output image
FROM scratch
COPY --from=build /home/myuser/build/src-tauri/target/release/bundle/*/*.AppImage /
COPY --from=build /home/myuser/build/src-tauri/target/release/bundle/*/*.deb /
COPY --from=build /home/myuser/build/src-tauri/target/release/bundle/*/*.rpm /
