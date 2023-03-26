# Nois multitest

Steps:

- Instantiate and check nois-icecube contract
- Instantiate and check nois-drand contract
- Set nois-drand address in nois-icecube
- Instantiate nois-proxy
- Register a bot in nois-drand
- Check that a non manager cannot allowlist a bot
- Allowlist the bot
- Add randomness round in nois-drand
- Check that the incentive has been paid from the icecube contract to the
  drand-operator by checking the respective Bank balances
- As manager, make the nois-icecube delegate to a validator and check that the
  delegations are queriable
