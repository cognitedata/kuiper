#! /bin/bash

if [ ! -d ./.env ]; then
    echo "Creating virtualenv"
    python -m venv .env
fi

source .env/bin/activate
maturin develop
