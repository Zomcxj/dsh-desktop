use crate::bootstrap::UiMsg;

pub fn build_splash_html() -> String {
    r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="utf-8">
<style>
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #000; color: #fff; font-family: "Segoe UI", system-ui, sans-serif; }
  main { width: 360px; text-align: center; }
  h1 { margin: 0 0 5px; font-size: 21px; font-weight: 600; }
  .sub { margin-bottom: 22px; color: #8b8b8b; font-size: 12px; }
  #spinner { width: 22px; height: 22px; margin: 0 auto 16px; border: 3px solid #303030; border-top-color: #4d6bfe; border-radius: 50%; animation: spin .8s linear infinite; }
  #status { min-height: 20px; margin-bottom: 12px; font-size: 13px; }
  .hidden { display: none; }
  .actions { display: flex; justify-content: center; gap: 8px; margin-top: 14px; }
  button { padding: 6px 18px; border: 1px solid #363636; border-radius: 5px; background: #191919; color: #fff; cursor: pointer; font: 12px "Segoe UI", system-ui, sans-serif; }
  button:hover { background: #292929; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
</head>
<body>
<main>
  <h1>DeepSeek Harness Desktop</h1>
  <div id="spinner"></div>
  <div id="status">正在初始化...</div>
  <div class="actions">
    <button id="retry" class="hidden" onclick="window.ipc.postMessage('retry')">重试</button>
    <button id="exit" class="hidden" onclick="window.ipc.postMessage('exit')">退出</button>
  </div>
</main>
<script>
  function setStatus(text) { document.getElementById('status').textContent = text; }
  function showFail(message) { setStatus(message); document.getElementById('spinner').classList.add('hidden'); document.getElementById('retry').classList.remove('hidden'); document.getElementById('exit').classList.remove('hidden'); }
  function showDone() { document.getElementById('spinner').classList.add('hidden'); }
  function reset() { setStatus('正在初始化...'); document.getElementById('spinner').classList.remove('hidden'); document.getElementById('retry').classList.add('hidden'); document.getElementById('exit').classList.add('hidden'); }
</script>
</body>
</html>"#
        .to_string()
}

pub fn ui_msg_js(message: &UiMsg) -> String {
    match message {
        UiMsg::Step(text) => format!("setStatus({text:?});"),
        UiMsg::Fail(error) => format!("showFail({error:?});"),
        UiMsg::Done(_) => "showDone();".into(),
    }
}

pub fn apply_msg(webview: &wry::WebView, message: &UiMsg) -> bool {
    webview.evaluate_script(&ui_msg_js(message)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_html_has_required_ids() {
        let html = build_splash_html();
        assert!(html.contains(r#"id="status""#));
        assert!(html.contains(r#"id="spinner""#));
    }

    #[test]
    fn ui_messages_produce_javascript() {
        assert!(ui_msg_js(&UiMsg::Step("检查端口".into())).contains("检查端口"));
        assert!(ui_msg_js(&UiMsg::Fail("出错了".into())).contains("出错了"));
    }
}
