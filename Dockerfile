FROM rust:1-bullseye

RUN apt update && apt install -y git make curl tar
RUN mkdir /opt/nvim
RUN curl -fsSL https://github.com/neovim/neovim/releases/download/v0.7.2/nvim-linux64.tar.gz -o /opt/nvim.tar.gz
RUN tar -xvzf /opt/nvim.tar.gz --strip-components=1 -C /opt/nvim
ENV PATH="/opt/nvim/bin:${PATH}"

RUN git config --global user.name "williambotman[bot]" && \
    git config --global user.email "william+bot@redwill.se"

WORKDIR /app
COPY . .
ENV ROCKET_ENV=production
RUN cargo build -r
RUN cp /app/target/release/botman /usr/local/bin/

EXPOSE 80

CMD [ "botman" ]
