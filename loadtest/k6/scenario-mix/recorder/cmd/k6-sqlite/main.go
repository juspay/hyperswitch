package main

import (
	"go.k6.io/k6/cmd"
	_ "hyperswitch.local/loadtest-recorder"
)

func main() { cmd.Execute() }
