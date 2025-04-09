package main

import (
	"fmt"
)

func main() {
	var x int64
	f(&x)
	fmt.Printf("%d\n", x)
}
