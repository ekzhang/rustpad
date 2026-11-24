# Packages artifacts built on the host by ./build.sh.
FROM scratch
ARG TARGETARCH
COPY dist dist
COPY build/linux-${TARGETARCH}/rustpad-server .
USER 1000:1000
CMD [ "./rustpad-server" ]
