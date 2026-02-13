#!/bin/bash

# Run 5 curl requests concurrently
for i in {1..100}
do
  curl http://localhost:8080/health/ready &
done

# Wait for all background processes to finish
wait