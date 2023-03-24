# Nois multitest

Steps:

- Instantiate and check nois-delegator contract
- Instantiate and check nois-oracle contract
- Set nois-oracle address in nois-delegator
- Instantiate nois-proxy
- Register a bot in nois-oracle
- Check that a non admin cannot whitelist a bot
- Whitelist the bot
- Add randomness round in nois-oracle
- Check that the incentive has been paid from the delegator contract to the
  drand-operator by checking the respective Bank balances
- As admin, make the nois-delegator delegate to a validator and check that the
  delegations are queriable
