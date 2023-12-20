#!/bin/bash
docker build . -t war_score 
docker compose down --remove-orphans
docker compose up -d
