package test

import (
	"os"
	"path/filepath"
	"strconv"
	"testing"
	"time"

	"istio.io/proxy/test/envoye2e/driver"
	"istio.io/proxy/test/envoye2e/env"
	"istio.io/proxy/testdata"
)

// Inventory is initialised at the bottom of the file. Port assignments will be
// offset based on the index of the test in the inventory.
var ExtensionE2ETests *env.TestInventory

func TestAuthn(t *testing.T) {
	var tests = []struct {
		name            string
		method          string
		path            string
		authority       string
		omitUpstream    bool
		wantErr         bool
		authnSuccess    int
		authnFailure    int
		requestHeaders  map[string]string
		responseHeaders map[string]string
		responseCode    int
	}{
		{
			name:            "CorrectCredentials",
			method:          "GET",
			path:            "/api",
			authority:       "authn.example.com",
			authnSuccess:    1,
			requestHeaders:  map[string]string{"Authorization": "Bearer correct-credentials"},
			responseHeaders: map[string]string{},
			responseCode:    200,
		},
		{
			name:            "UnregisteredBackend",
			method:          "GET",
			path:            "/bad",
			authority:       "authn.example.com",
			authnSuccess:    1,
			requestHeaders:  map[string]string{"Authorization": "Bearer correct-credentials"},
			responseHeaders: map[string]string{},
			responseCode:    404,
		},
		{
			name:            "IncorrectCredentials",
			method:          "GET",
			path:            "/api",
			authority:       "authn.example.com",
			authnFailure:    1,
			requestHeaders:  map[string]string{"Authorization": "Bearer incorrect-credentials"},
			responseHeaders: map[string]string{},
			responseCode:    401,
		},
		{
			name:            "MissingCredentials",
			method:          "GET",
			path:            "/api",
			authnFailure:    1,
			responseHeaders: map[string]string{},
			responseCode:    200,
		},
		{
			name:            "Misconfiguration",
			method:          "GET",
			path:            "/api",
			omitUpstream:    true,
			wantErr:         true,
			responseHeaders: map[string]string{},
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			params := driver.NewTestParams(t, map[string]string{
				"DisableDirectResponse": "true",
				"OmitUpstream":          strconv.FormatBool(tt.omitUpstream),
				"ServerStaticCluster":   driver.LoadTestData("tests/testdata/cluster/authn_server.yaml.tmpl"),
				"ServerMetadata":        driver.LoadTestData("tests/testdata/server_node_metadata.yaml.tmpl"),
				"Authority":             tt.authority,
				"AuthnWasmFile":         filepath.Join(env.GetBazelBinOrDie(), "token_authn.wasm"),
				"AuthnSuccess":          strconv.Itoa(tt.authnSuccess),
				"AuthnFailure":          strconv.Itoa(tt.authnFailure),
			}, ExtensionE2ETests)

			params.Vars["ServerHTTPFilters"] = params.LoadTestData("tests/testdata/server_filter.yaml.tmpl")

			scenario := &driver.Scenario{
				Steps: []driver.Step{
					&driver.XDS{},
					&driver.Update{
						Node:    "server",
						Version: "0",
						Listeners: []string{
							string(testdata.MustAsset("listener/server.yaml.tmpl")),
							params.LoadTestData("tests/testdata/listener/staticreply.yaml.tmpl"),
						},
					},
					&driver.Envoy{
						Bootstrap:       params.FillTestData(string(testdata.MustAsset("bootstrap/server.yaml.tmpl"))),
						DownloadVersion: os.Getenv("ISTIO_TEST_VERSION"),
					},
					&driver.Sleep{Duration: 1 * time.Second},
					&driver.HTTPCall{
						Port:            params.Ports.ServerPort,
						Method:          tt.method,
						Path:            tt.path,
						RequestHeaders:  tt.requestHeaders,
						ResponseCode:    tt.responseCode,
						ResponseHeaders: tt.responseHeaders,
					},
					&driver.Stats{
						AdminPort: params.Ports.ServerAdmin,
						Matchers: map[string]driver.StatMatcher{
							"envoy_token_authn_filter_authentication_success_count": &driver.ExactStat{
								Metric: "tests/testdata/stats/authn_success.yaml.tmpl",
							},
							"envoy_token_authn_filter_authentication_failure_count": &driver.ExactStat{
								Metric: "tests/testdata/stats/authn_failure.yaml.tmpl",
							},
						},
					},
				},
			}

			if err := scenario.Run(params); (err != nil) != tt.wantErr {
				t.Fatal(err)
			}
		})
	}
}

func init() {
	ExtensionE2ETests = &env.TestInventory{
		Tests: []string{
			"TestAuthn/CorrectCredentials",
			"TestAuthn/UnregisteredBackend",
			"TestAuthn/IncorrectCredentials",
			"TestAuthn/MissingCredentials",
			"TestAuthn/Misconfiguration",
		},
	}
}
