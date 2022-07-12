FROM rust:1-bullseye AS botman-builder

WORKDIR /app
COPY . .
RUN cargo build -r

FROM rust:1-bullseye

# Install & setup Neovim
RUN apt update && apt install -y git make curl tar
RUN mkdir /opt/nvim
RUN curl -fsSL https://github.com/neovim/neovim/releases/download/v0.7.2/nvim-linux64.tar.gz -o /opt/nvim.tar.gz
RUN tar -xvzf /opt/nvim.tar.gz --strip-components=1 -C /opt/nvim
ENV PATH="/opt/nvim/bin:${PATH}"

RUN git clone --depth 1 https://github.com/williamboman/mason.nvim ~/.local/share/nvim/site/pack/vendor/start/mason.nvim
ENV PATH="~/.local/share/nvim/mason/bin:${PATH}"
RUN mkdir -p ~/.config/nvim && echo 'require("mason").setup()' > ~/.config/nvim/init.lua

# Install runtime deps
RUN cargo install stylua

# Configure git
RUN git config --global user.name "williambotman[bot]" && \
    git config --global user.email "william+bot@redwill.se"

WORKDIR /app
COPY --from=botman-builder /app/target/release/botman /usr/local/bin/botman
ENV ROCKET_ENV=production

EXPOSE 80

CMD [ "botman" ]
