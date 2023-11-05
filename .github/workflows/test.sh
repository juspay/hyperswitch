#! /bin/bash

if git diff --quiet && git diff --staged --quiet; then
            echo "No changes detected."
          else 
            if git add postman && ! git diff --staged --quiet postman; then
              git commit -am 'test(postman): update postman collection files'
              echo "Changes detected and commited."
          fi