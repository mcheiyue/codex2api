#!/usr/bin/env bash
set -e

git pull
cd frontend
npm run build
cd ..
go build -o codex2api .
systemctl daemon-reload
systemctl restart codex2api
