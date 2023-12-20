#!/bin/bash
docker build . -t war_overlay 
docker-compose down --remove-orphans
docker-compose up -d
