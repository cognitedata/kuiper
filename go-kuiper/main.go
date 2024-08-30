package main

import (
	"fmt"
	"log"

	"github.com/cognitedata/go-kuiper/kuiper"
)

func main() {
	// Create a new Kuiper expression
	expression := "in1 + in2.test"
	fmt.Printf("Creating Kuiper expression: %s\n", expression)
	expr, err := kuiper.NewKuiperExpression(expression, []string{"in1", "in2"})
	if err != nil {
		log.Fatalf("Failed to create expression: %v", err)
	}
	defer expr.Dispose()

	// Apply the expression to some data
	data1 := "1"
	data2 := `{"test": 2}`
	fmt.Printf("Running Kuiper expression with data: %s, %s\n", data1, data2)
	result, err := expr.Run(data1, data2)
	if err != nil {
		log.Fatalf("Failed to run expression: %v", err)
	}

	fmt.Printf("Result: %s\n", result)
}
