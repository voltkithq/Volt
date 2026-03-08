use super::*;

#[test]
fn http_module_fetch_exposes_status_text_and_json() {
    let (url, server) = spawn_http_fixture_server("{\"ok\":true}");
    let runtime = runtime_with_permissions(unique_temp_dir("http-module"), &["http"]);

    let script = format!(
        "(async () => {{
                const http = globalThis.__volt.http;
                const response = await http.fetch('{url}', {{ method: 'GET' }});
                const text = await response.text();
                const json = await response.json();
                return `${{response.status}}:${{text}}:${{json.ok}}`;
            }})()"
    );

    let summary = runtime
        .client()
        .eval_promise_string(script.as_str())
        .expect("http fetch script");
    assert_eq!(summary, "200:{\"ok\":true}:true");

    let _ = server.join();
}

#[test]
fn http_module_accepts_request_object_api() {
    let (url, server) = spawn_http_fixture_server("{\"ok\":true}");
    let runtime = runtime_with_permissions(unique_temp_dir("http-object"), &["http"]);

    let script = format!(
        "(async () => {{
                const http = globalThis.__volt.http;
                const response = await http.fetch({{
                    url: '{url}',
                    method: 'GET',
                    timeoutMs: 1000
                }});
                const text = await response.text();
                return `${{response.status}}:${{text}}`;
            }})()"
    );

    let summary = runtime
        .client()
        .eval_promise_string(script.as_str())
        .expect("http request object script");
    assert_eq!(summary, "200:{\"ok\":true}");

    let _ = server.join();
}

#[test]
fn http_module_rejects_invalid_method_before_request() {
    let runtime = runtime_with_permissions(unique_temp_dir("http-method"), &["http"]);

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const http = globalThis.__volt.http;
                    try {
                        await http.fetch('http://127.0.0.1:1', { method: 'NOT A METHOD' });
                        return 'unexpected';
                    } catch (_) {
                        return 'invalid';
                    }
                })()",
        )
        .expect("http invalid method script");
    assert_eq!(outcome, "invalid");
}

#[test]
fn http_module_rejects_without_permission() {
    let runtime = JsRuntimeManager::start().expect("js runtime start");

    let outcome = runtime
        .client()
        .eval_promise_string(
            "(async () => {
                    const http = globalThis.__volt.http;
                    try {
                        await http.fetch('https://example.com');
                        return 'unexpected';
                    } catch (error) {
                        return String(error).includes('Permission denied') ? 'denied' : String(error);
                    }
                })()",
        )
        .expect("http permission script");
    assert_eq!(outcome, "denied");
}

#[test]
fn http_module_preserves_duplicate_response_headers() {
    let (url, server) = spawn_http_fixture_server_with_duplicate_headers();
    let runtime = runtime_with_permissions(unique_temp_dir("http-headers"), &["http"]);

    let script = format!(
        "(async () => {{
                const http = globalThis.__volt.http;
                const response = await http.fetch('{url}');
                const cookies = response.headers['set-cookie'];
                return `${{Array.isArray(cookies)}}:${{cookies.length}}:${{cookies[0]}}:${{cookies[1]}}`;
            }})()"
    );

    let summary = runtime
        .client()
        .eval_promise_string(script.as_str())
        .expect("http headers script");
    assert_eq!(summary, "true:2:a=1:b=2");

    let _ = server.join();
}
