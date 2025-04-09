package main

import (
	"flag"
	"fmt"
)

type T1 = uint32
type T2 uint64

type T3 struct {
	field   int32
	t4field *T4
}

type J interface{}

type (
	I interface {
		f()
	}

	T4 struct {
		field int32
		i     J
	}
)

const X = 1337

const (
	Y = 69
	Z = 420
)

var chrootDir = flag.String("chroot", "", "chroot before scanning")

var (
	OtherFlag = flag.String("other", "", "other")
)

func main() {
	flag.Parse()
	var t T3
	var t2 T4
	t.field = 1
	t2.field = 2
	fmt.Printf("%s %d %d\n", *chrootDir, t.field, t2.field)
}
