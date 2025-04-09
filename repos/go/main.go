package main

import (
	"fmt"
)

func main() {
	var x int64
	f(&x)
	fmt.Printf("%d\n", x)
}

func f(x *int64) {
	*x = 10
}
