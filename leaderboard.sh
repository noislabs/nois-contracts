number_of_bots=$(wasmd query wasm  contract-state  smart nois1j7m4f68lruceg5xq3gfkfdgdgz02vhvlq2p67vf9v3hwdydaat3sajzcy5  '{"bots": {}}'  --node=http://node-0.noislabs.com:26657 |yq -r '.data.bots | length')
for bot in `seq 0 $(($number_of_bots - 1))` 
 do
  wasmd query wasm  contract-state  smart nois1j7m4f68lruceg5xq3gfkfdgdgz02vhvlq2p67vf9v3hwdydaat3sajzcy5  '{"bots": {}}'  --node=http://node-0.noislabs.com:26657 |yq -r ".data.bots[$bot].moniker"
  wasmd query wasm  contract-state  smart nois1j7m4f68lruceg5xq3gfkfdgdgz02vhvlq2p67vf9v3hwdydaat3sajzcy5  '{"bots": {}}'  --node=http://node-0.noislabs.com:26657 |yq -r ".data.bots[$bot].rounds_added"
done
