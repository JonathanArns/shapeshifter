package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/http/httputil"
	"net/url"
	"time"
)

// NewProxy takes target host and creates a reverse proxy
func NewProxy(targetHost string) (*httputil.ReverseProxy, error) {
	url, err := url.Parse(targetHost)
	if err != nil {
		return nil, err
	}

	proxy := httputil.NewSingleHostReverseProxy(url)

	originalDirector := proxy.Director
	proxy.Director = func(req *http.Request) {
		originalDirector(req)
		modifyRequest(req)
	}

	proxy.ErrorHandler = errorHandler()
	return proxy, nil
}

func max(x, y int) int {
	if x < y {
		return y
	}
	return x
}

func modifyRequest(req *http.Request) {
	if req.Method == "POST" && req.URL.Path == "/move" {
		buf := &bytes.Buffer{}
		teeReader := io.TeeReader(req.Body, buf)
		decoder := json.NewDecoder(teeReader)
		var state map[string]interface{}
		decoder.Decode(&state)
		req.Body = io.NopCloser(buf)
		timeout := int(state["game"].(map[string]interface{})["timeout"].(float64))
		duration, _ := time.ParseDuration(fmt.Sprint(max(timeout/2, timeout-100), "ms"))
		deadline := time.Now().Add(duration)
		req.Header.Set("x-deadline-unix-millis", fmt.Sprint("", deadline.UnixMilli()))
	}
}

func errorHandler() func(http.ResponseWriter, *http.Request, error) {
	return func(w http.ResponseWriter, req *http.Request, err error) {
		fmt.Printf("Got error while modifying response: %v \n", err)
		return
	}
}

// ProxyRequestHandler handles the http request using proxy
func ProxyRequestHandler(proxy *httputil.ReverseProxy) func(http.ResponseWriter, *http.Request) {
	return func(w http.ResponseWriter, r *http.Request) {
		proxy.ServeHTTP(w, r)
	}
}

func main() {
	// initialize a reverse proxy and pass the actual backend server url here
	proxy, err := NewProxy("http://localhost:8080")
	if err != nil {
		panic(err)
	}

	// handle all requests to your server using the proxy
	http.HandleFunc("/", ProxyRequestHandler(proxy))
	log.Fatal(http.ListenAndServe(":8000", nil))
}
