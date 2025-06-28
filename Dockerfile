FROM ubuntu:24.04 AS build

RUN apt-get update -y
RUN apt-get upgrade -y

RUN apt-get install -y rustup libwebkit2gtk-4.1-dev curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev nodejs npm libasound2-dev

RUN useradd -ms /bin/bash myuser
USER myuser

# Install Rust
RUN rustup default 1.88

# Copy tauri directory into container
WORKDIR /home/myuser/tauri
ADD --chown=myuser tauri .

RUN npm install
RUN npm run tauri build

# Copy built packages into output image
FROM scratch
COPY --from=build /home/myuser/tauri/src-tauri/target/release/bundle/*/*.AppImage /
COPY --from=build /home/myuser/tauri/src-tauri/target/release/bundle/*/*.deb /
COPY --from=build /home/myuser/tauri/src-tauri/target/release/bundle/*/*.rpm /
