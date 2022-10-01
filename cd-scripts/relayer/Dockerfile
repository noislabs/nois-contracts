#The confio confio/relayer npm is used by Nois and is developed by confio in this repo https://github.com/confio/ts-relayer.git
#WARN!! This is only for testing the relayers not for production use


FROM node:18.8

ARG CHAIN_NAME
ARG NOIS_CHAIN_NAME

RUN npm i -g @confio/relayer@0.6.1
COPY nois-relayer-config.yaml /root/.ibc-setup/registry.yaml 
RUN ibc-setup init --src $CHAIN_NAME --dest $NOIS_CHAIN_NAME

CMD ["ibc-relayer", "start", "--src=uni", "--dest=nois", "--src-connection=connection-262", "--dest-connection=connection-1"]
