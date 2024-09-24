package main

import (
	"bytes"
	"embed"
	"encoding/json"
	"html/template"
	"io"
	"log"
	"net/http"

	"github.com/cognitedata/go-kuiper/kuiper"
)

//go:embed templates
var templateFS embed.FS

var indexTmpl = template.Must(template.ParseFS(templateFS, "templates/index.html"))

func main() {
	http.HandleFunc("/", handleIndex)
	http.HandleFunc("/evaluate", handleEvaluate)
	http.HandleFunc("/graphql", handleGraphQL)

	log.Println("Server starting on http://localhost:8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

func handleIndex(w http.ResponseWriter, r *http.Request) {
	indexTmpl.Execute(w, nil)
}

func handleEvaluate(w http.ResponseWriter, r *http.Request) {
	var input struct {
		JSONData   string `json:"jsonData"`
		Expression string `json:"expression"`
	}

	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		renderError(w, "Failed to parse JSON input: "+err.Error(), http.StatusBadRequest)
		return
	}

	expr, err := kuiper.NewKuiperExpression(input.Expression, []string{"input"})
	if err != nil {
		renderError(w, "Failed to create expression: "+err.Error(), http.StatusBadRequest)
		return
	}
	defer expr.Dispose()

	result, err := expr.Run(input.JSONData)
	if err != nil {
		renderError(w, "Failed to run expression: "+err.Error(), http.StatusInternalServerError)
		return
	}

	var prettyResult interface{}
	err = json.Unmarshal([]byte(result), &prettyResult)
	if err != nil {
		prettyResult = result
	}

	prettyJSON, err := json.MarshalIndent(prettyResult, "", "  ")
	if err != nil {
		renderError(w, "Failed to format result: "+err.Error(), http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.Write(prettyJSON)
}

func handleGraphQL(w http.ResponseWriter, r *http.Request) {
	var input struct {
		URL   string `json:"url"`
		Query string `json:"query"`
	}

	if err := json.NewDecoder(r.Body).Decode(&input); err != nil {
		renderError(w, "Failed to parse JSON input: "+err.Error(), http.StatusBadRequest)
		return
	}

	// Create GraphQL request payload
	payload := map[string]interface{}{
		"query": input.Query,
	}
	payloadBytes, err := json.Marshal(payload)
	if err != nil {
		renderError(w, "Failed to create GraphQL payload: "+err.Error(), http.StatusInternalServerError)
		return
	}

	// Create GraphQL request
	graphqlReq, err := http.NewRequest("POST", input.URL, bytes.NewBuffer(payloadBytes))
	if err != nil {
		renderError(w, "Failed to create GraphQL request: "+err.Error(), http.StatusInternalServerError)
		return
	}

	// Set headers
	graphqlReq.Header.Set("Content-Type", "application/json")
	// Use the Authorization header from the incoming request
	authHeader := r.Header.Get("Authorization")
	if authHeader != "" {
		graphqlReq.Header.Set("Authorization", authHeader)
	}

	// Send request
	client := &http.Client{}
	resp, err := client.Do(graphqlReq)
	if err != nil {
		renderError(w, "Failed to send GraphQL request: "+err.Error(), http.StatusInternalServerError)
		return
	}
	defer resp.Body.Close()

	// Read response
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		renderError(w, "Failed to read GraphQL response: "+err.Error(), http.StatusInternalServerError)
		return
	}

	// Send response back to client
	w.Header().Set("Content-Type", "application/json")
	w.Write(body)
}

func renderError(w http.ResponseWriter, message string, status int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]string{"error": message})
}
