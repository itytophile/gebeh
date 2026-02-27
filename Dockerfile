FROM node:alpine AS front

WORKDIR /app

# don't use wasm-pack from npm, it doesn't seem to work well

RUN apk add rustup clang curl &&\
    rustup-init -y --profile minimal &&\
    . "$HOME/.cargo/env" &&\
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | ash &&\
    cargo binstall wasm-pack

COPY . .

WORKDIR /app/gebeh-web

RUN . "$HOME/.cargo/env" && npm ci && npm run build &&\
    mkdir page && cp -R assets/ dist/ pkg/ polyfill/ index.html style.css manifest.json page/

FROM docker.io/clux/muslrust:stable AS back

WORKDIR /app

COPY . .

RUN cargo build --release -p gebeh-server

FROM scratch

COPY --from=back /app/target/x86_64-unknown-linux-musl/release/gebeh-server .
COPY --from=front /app/gebeh-web/page /page

CMD [ "/gebeh-server", "page" ]
