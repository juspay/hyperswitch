package main

import (
	"go.k6.io/k6/cmd"
	_ "hyperswitch.local/loadtest-recorder"
)

// main runs the standard k6 CLI with the recorder registered by the blank
// import, enabling scenarios to select the sqlite-recorder output.
func main() { cmd.Execute() }
