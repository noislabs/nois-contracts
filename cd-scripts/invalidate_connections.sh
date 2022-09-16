#!/bin/bash

#invalidate all connections
sed -i ''  's/src:.*$/src: ~/g' cd-scripts/config.yaml 
sed -i ''  's/dest:.*$/dest: ~/g' cd-scripts/config.yaml 