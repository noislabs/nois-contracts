#!/bin/bash

#invalidate all addresses to redeploy
sed -i ''  's/address: .*$/address:/g' cd-scripts/config.yaml 