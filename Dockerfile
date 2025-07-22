#syntax=docker/dockerfile:1.4
FROM scottyhardy/docker-wine:stable

RUN mkdir /home/wineuser

COPY ./.wine /home/wineuser/.wine
