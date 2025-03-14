FROM node:lts

RUN npm install concurrently -g

WORKDIR /hyperswitch-web

RUN git clone https://github.com/juspay/hyperswitch-web --depth 1 .

RUN npm install

EXPOSE 9050
EXPOSE 5252
EXPOSE 9060

CMD concurrently "npm run re:build && npm run start" "npm run start:playground"