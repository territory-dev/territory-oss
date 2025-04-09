package main

import "fmt"

type I interface {
	f() int
}

type T struct{}

func (t *T) f() int {
	return 100
}

func main() {
	var i I
	i = &T{}
	fmt.Printf("%d\n", i.f())
}
