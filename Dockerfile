FROM node:alpine AS front

WORKDIR /app

RUN apk add rustup clang &&\
    rustup-init -y --profile minimal &&\
    npm install -g wasm-pack

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
COPY --from=front /app/gebeh-web/page .

CMD [ "/gebeh-server", "page" ]
