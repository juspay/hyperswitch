FROM node:lts-alpine

RUN npm install concurrently -g

WORKDIR /hyperswitch-web

RUN git clone https://github.com/juspay/hyperswitch-web .

RUN sed -i '/hot: true,/a \  host: "0.0.0.0",' webpack.dev.js
RUN sed -i '/hot: true,/a \  host: "0.0.0.0",' Hyperswitch-React-Demo-App/webpack.dev.js

RUN npm install

EXPOSE 9050
EXPOSE 5252
EXPOSE 9060

CMD concurrently "npm run start:dev" "npm run start:playground"