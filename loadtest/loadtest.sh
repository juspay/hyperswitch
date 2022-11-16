#!/bin/bash

run_loadtest() {
  docker compose down
  docker compose up k6
}

print_details() {
  echo "-----------------------------------"
  echo "RUN NAME: $1"
  echo "  SCRIPT: $2"
  echo "-----------------------------------"
}

exit_if_baseline_file_not_exist() {
  if ! [ -e "./k6/benchmark/baseline_$1.json" ]
  then
    echo "baseline_$1.json file not exist to compare."
    exit 1
  fi
}

while getopts r:s:ca flag
do
  case "${flag}" in
    r) run_name=${OPTARG};;
    s) script=${OPTARG};;
    c) compare=true;;
    a) all_script=true;;
    *) echo "usage: $0 [-r] [-c] [-s]" >&2
       exit 1;;
  esac
done

# if script is empty, `-s` not specified, use "health"
if [ -z "$script" ]
then
  script=health
fi

if ! [ -e "./k6/$script.js" ] && [ -z "$all_script" ]
then
  echo "$script.js not exist."
  exit 1
fi

# if compare is specified using `-c` flag, create run name using commit number
if [ "$compare" = true ]
then
  run_name=$(git show -s --format=%h)
  # make sure baseline file exist for specified parameter before starting the loadtest
  if [ "$all_script" = true ]
  then
    for scriptname in ./k6/*.js
    do
      filename=$(basename "$scriptname" | cut -f 1 -d '.')
      exit_if_baseline_file_not_exist "$filename"
    done
  else
    exit_if_baseline_file_not_exist "$script"
  fi
else
  # if run name is empty, `-r` not specified, use "baseline"
  if [ -z "$run_name" ]
  then
    run_name=baseline
  fi
fi

export LOADTEST_RUN_NAME=$run_name

if [ "$all_script" = true ]
then
  for script in ./k6/*.js
  do
    script=$(basename "$script")
    print_details "$run_name" "$script"
    export LOADTEST_K6_SCRIPT=$script
    run_loadtest
  done
else
  print_details "$run_name" "$script.js"
  export LOADTEST_K6_SCRIPT="$script.js"
  run_loadtest
fi
