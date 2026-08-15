use crate::bootstrap::UiMsg;

pub fn build_splash_html() -> String {
    r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="utf-8">
<style>
  * { box-sizing: border-box; }
  body { margin: 0; min-height: 100vh; background: #000; color: #fff; font-family: "Segoe UI", system-ui, sans-serif; }

  /* 启动页面 */
  #splash { display: grid; place-items: center; min-height: 100vh; }
  #splash main { width: 360px; text-align: center; }
  h1 { margin: 0 0 5px; font-size: 21px; font-weight: 600; }
  .sub { margin-bottom: 22px; color: #8b8b8b; font-size: 12px; }
  #spinner { width: 22px; height: 22px; margin: 0 auto 16px; border: 3px solid #303030; border-top-color: #4d6bfe; border-radius: 50%; animation: spin .8s linear infinite; }
  #status { min-height: 20px; margin-bottom: 12px; font-size: 13px; }
  .hidden { display: none !important; }
  .actions { display: flex; justify-content: center; gap: 8px; margin-top: 14px; }
  button { padding: 6px 18px; border: 1px solid #363636; border-radius: 5px; background: #191919; color: #fff; cursor: pointer; font: 12px "Segoe UI", system-ui, sans-serif; }
  button:hover { background: #292929; }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
</head>
<body>
<div id="splash">
<main>
  <h1>DeepSeek Harness Desktop</h1>
  <div class="sub">DeepSeek Harness</div>
  <div id="spinner"></div>
  <div id="status">正在初始化...</div>
  <div class="actions">
    <button id="retry" class="hidden" onclick="window.ipc.postMessage('retry')">重试</button>
    <button id="exit" class="hidden" onclick="window.ipc.postMessage('exit')">退出</button>
  </div>
</main>
</div>
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

/// 注入导航栏到 DSH 页面。每次页面加载后调用。
pub fn inject_navbar_script() -> String {
    r#"(function(){
  if (document.getElementById('dsh-navbar')) return;
  var nav = document.createElement('div');
  nav.id = 'dsh-navbar';
  nav.innerHTML = '<style>'
    + '#dsh-navbar{position:fixed;top:0;left:0;right:0;height:20px;background:transparent;z-index:2147483647;display:flex;align-items:center;padding:0 6px;gap:3px;font-family:"Segoe UI",system-ui,sans-serif;pointer-events:none;}'
    + '#dsh-navbar .n-btn{width:18px;height:16px;border:none;border-radius:3px;background:transparent;cursor:pointer;display:flex;align-items:center;justify-content:center;position:relative;padding:0;pointer-events:auto;}'
    + '#dsh-navbar .n-btn:hover{background:#333;}'
    + '#dsh-navbar .n-btn.active{background:#4d6bfe;}'
    + '#dsh-navbar .n-btn svg{width:12px;height:12px;fill:none;stroke:#888;stroke-width:2;stroke-linecap:round;stroke-linejoin:round;}'
    + '#dsh-navbar .n-btn:hover svg{stroke:#fff;}'
    + '#dsh-navbar .n-btn.active svg{stroke:#fff;}'
    + '#dsh-navbar .n-tip{position:absolute;top:100%;left:50%;transform:translateX(-50%);margin-top:2px;padding:2px 6px;background:#333;color:#fff;font-size:10px;border-radius:3px;white-space:nowrap;pointer-events:none;opacity:0;transition:opacity .15s;z-index:2147483647;}'
    + '#dsh-navbar .n-btn:hover .n-tip{opacity:1;}'
    + '#dsh-navbar .n-spacer{flex:1;}'
    + '</style>'
    + '<button class="n-btn">'
      + '<svg viewBox="0 0 24 24"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/></svg>'
      + '<span class="n-tip">设置</span>'
    + '</button>'
    + '<button class="n-btn" onclick="window.ipc.postMessage(\'refresh\')">'
      + '<svg viewBox="0 0 24 24"><path d="M2 12C2 6.48 6.48 2 12 2s10 4.48 10 10-4.48 10-10 10S2 17.52 2 12z"/><path d="M12 6v6l4 2"/></svg>'
      + '<span class="n-tip">刷新页面</span>'
    + '</button>'
    + '<button class="n-btn" onclick="window.ipc.postMessage(\'restart\')">'
      + '<svg viewBox="0 0 24 24"><path d="M3 12a9 9 0 1 1 9 9"/><path d="M3 3v6h6"/></svg>'
      + '<span class="n-tip">重启服务</span>'
    + '</button>'
    + '<button class="n-btn" id="dsh-btn-exit" onclick="window.ipc.postMessage(\'toggle-exit-mode\')">'
      + '<svg viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>'
      + '<span class="n-tip">关闭时退出</span>'
    + '</button>'
    + '<button class="n-btn" id="dsh-btn-tray" onclick="window.ipc.postMessage(\'toggle-tray\')">'
      + '<svg viewBox="0 0 24 24"><rect x="4" y="2" width="16" height="20" rx="2"/><line x1="9" y1="6" x2="15" y2="6"/></svg>'
      + '<span class="n-tip">常驻任务栏</span>'
    + '</button>'
    + '<span class="n-spacer"></span>';
  document.body.appendChild(nav);
})();"#
        .to_string()
}

/// 更新导航栏按钮状态
pub fn nav_set_exit_mode(enabled: bool) -> String {
    let active_class = if enabled { " active" } else { "" };
    let tip = if enabled { "关闭时退出 ✓" } else { "关闭时隐藏" };
    format!(
        "(function(){{var b=document.getElementById('dsh-btn-exit');if(b){{b.className='n-btn{ac}'}};
        var t=b?b.querySelector('.n-tip'):null;if(t)t.textContent='{tip}';}})();",
        ac = active_class,
        tip = tip
    )
}

pub fn nav_set_tray_mode(enabled: bool) -> String {
    let active_class = if enabled { " active" } else { "" };
    let tip = if enabled { "常驻任务栏 ✓" } else { "不驻任务栏" };
    format!(
        "(function(){{var b=document.getElementById('dsh-btn-tray');if(b){{b.className='n-btn{ac}'}};
        var t=b?b.querySelector('.n-tip'):null;if(t)t.textContent='{tip}';}})();",
        ac = active_class,
        tip = tip
    )
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
