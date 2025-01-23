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

	// Test with a GraphQL response
	jsonData := `{
  "data": {
    "listContinent": {
      "items": [
        {"name": "Asia"},
        {"name": "South America"},
        {"name": "North America"},
        {"name": "Africa"},
        {"name": "Europe"},
        {"name": "Oceania"}
      ]
    }
  }
}`

	fmt.Println("\nTesting JSON flattening with Kuiper...")
	fmt.Println("Input JSON data:")
	fmt.Println(jsonData)

	// Create a new Kuiper expression for flattening
	flattenExpr := "input.data.listContinent.items.map(item => item.name)"
	fmt.Printf("\nCreating Kuiper expression: %s\n", flattenExpr)
	expr, err = kuiper.NewKuiperExpression(flattenExpr, []string{"input"})
	if err != nil {
		log.Fatalf("Failed to create flattening expression: %v", err)
	}
	defer expr.Dispose()

	// Run the expression
	fmt.Println("Running Kuiper expression to flatten JSON...")
	result, err = expr.Run(jsonData)
	if err != nil {
		log.Fatalf("Failed to run flattening expression: %v", err)
	}

	// Print the result
	fmt.Println("Result from Kuiper expression:")
	fmt.Println(result)
}
