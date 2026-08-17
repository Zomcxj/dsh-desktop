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
  #splash { display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; gap: 12px; }
  #splash main { width: 360px; text-align: center; }
  h1 { margin: 0 0 5px; font-size: 21px; font-weight: 600; display: flex; align-items: center; justify-content: center; gap: 10px; }
  .title-icon { width: 24px; height: 24px; flex-shrink: 0; display: inline-block; background-image: url("data:image/png;base64,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"); background-size: contain; background-repeat: no-repeat; background-position: center; }
  .sub { margin-bottom: 22px; color: #8b8b8b; font-size: 12px; }
  #spinner { width: 22px; height: 22px; margin: 0 auto 16px; border: 3px solid #303030; border-top-color: #4d6bfe; border-radius: 50%; animation: spin .8s linear infinite; }
  #status { min-height: 20px; margin-bottom: 12px; font-size: 13px; }
  .hidden { display: none !important; }
  .actions { display: flex; justify-content: center; gap: 8px; margin-top: 14px; }
  button { padding: 6px 18px; border: 1px solid #363636; border-radius: 5px; background: #191919; color: #fff; cursor: pointer; font: 12px "Segoe UI", system-ui, sans-serif; }
  button:hover { background: #292929; }
  button:disabled { opacity: .45; cursor: not-allowed; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes rowIn { from { opacity: 0; transform: translateY(8px); } to { opacity: 1; transform: translateY(0); } }
  @keyframes checkingPulse { 0%,100% { opacity: .35; } 50% { opacity: 1; } }

  /* 环境检查面板 */
  #env-panel { display: none; width: 420px; max-width: 92vw; margin: 0 auto; text-align: left; background: #121212; border: 1px solid #2a2a2a; border-radius: 8px; padding: 14px 16px; }
  #env-panel h2 { margin: 0 0 12px; font-size: 14px; color: #ddd; }
  .env-row { padding: 8px 10px; border-radius: 6px; margin-bottom: 8px; background: #1a1a1a; animation: rowIn .3s ease both; }
  .env-row.bad { border: 1px solid #7a3030; }
  .env-row.ok { border: 1px solid #2a4a2a; }
  .env-row.checking { border: 1px dashed #4d6bfe; animation: none; }
  .env-row.checking .env-name { animation: checkingPulse 1.2s ease-in-out infinite; color: #4d6bfe; }
  .env-name { font-weight: 600; color: #eee; font-size: 13px; }
  .env-status { float: right; font-size: 12px; }
  .env-row.ok .env-status { color: #4caf50; }
  .env-row.bad .env-status { color: #e57373; }
  .env-row.checking .env-status { color: #4d6bfe; }
  .env-hint { margin-top: 6px; color: #999; font-size: 11px; line-height: 1.5; word-break: break-all; }
  .env-btn { margin-top: 8px; padding: 3px 12px; border: 1px solid #4d6bfe; border-radius: 4px; background: #2a3560; color: #fff; cursor: pointer; font: 11px "Segoe UI", system-ui, sans-serif; }
  .env-btn:hover { background: #3a4a80; }
  .env-btn:disabled { opacity: .5; cursor: wait; }
</style>
</head>
<body>
<div id="splash">
<main>
  <h1><span class="title-icon"></span>DeepSeek Harness Desktop</h1>

  <div id="spinner"></div>
  <div id="status">正在初始化...</div>
</main>

<!-- 环境检查面板：独立于 360px 的 main，居中显示 -->
<div id="env-panel">
  <h2>运行环境检查</h2>
  <div id="env-list"></div>
</div>
<div class="actions" id="env-actions">
  <button id="retry" class="hidden" onclick="window.ipc.postMessage('retry')">重试</button>
  <button id="exit" class="hidden" onclick="window.ipc.postMessage('exit')">退出</button>
</div>
</div>
<script>
  function setStatus(text) { document.getElementById('status').textContent = text; }
  function showFail(message) { setStatus(message); document.getElementById('spinner').classList.add('hidden'); document.getElementById('retry').classList.remove('hidden'); document.getElementById('exit').classList.remove('hidden'); }
  function showDone() { document.getElementById('spinner').classList.add('hidden'); }
  function reset() { setStatus('正在初始化...'); document.getElementById('spinner').classList.remove('hidden'); document.getElementById('retry').classList.add('hidden'); document.getElementById('exit').classList.add('hidden'); document.getElementById('env-panel').style.display = 'none'; document.getElementById('env-list').innerHTML = ''; }

  function showEnvProgress(name) {
    var panel = document.getElementById('env-panel');
    var list = document.getElementById('env-list');
    panel.style.display = 'block';
    // 清除前一行 checking 状态
    Array.prototype.forEach.call(list.querySelectorAll('.checking'), function (r) {
      r.parentNode.removeChild(r);
    });
    var row = document.createElement('div');
    row.className = 'env-row checking';
    var nameSpan = document.createElement('span');
    nameSpan.className = 'env-name';
    nameSpan.textContent = name;
    var status = document.createElement('span');
    status.className = 'env-status';
    status.textContent = '正在检查...';
    row.appendChild(nameSpan);
    row.appendChild(status);
    list.appendChild(row);
  }

  function showEnvCheck(results) {
    var panel = document.getElementById('env-panel');
    var list = document.getElementById('env-list');
    list.innerHTML = '';
    var allOk = true;
    results.forEach(function (r, i) {
      var row = document.createElement('div');
      row.className = 'env-row ' + (r.ok ? 'ok' : 'bad');
      row.style.animationDelay = (i * 120) + 'ms';
      var name = document.createElement('span');
      name.className = 'env-name';
      name.textContent = r.name;
      var status = document.createElement('span');
      status.className = 'env-status';
      status.textContent = r.ok ? ('✓ ' + (r.version || '已安装')) : '✗ ' + (r.error || '未安装');
      row.appendChild(name);
      row.appendChild(status);
      if (!r.ok) {
        allOk = false;
        var hint = document.createElement('div');
        hint.className = 'env-hint';
        hint.textContent = '安装方式: ' + r.install_hint;
        row.appendChild(hint);
        var btn = document.createElement('button');
        btn.className = 'env-btn';
        btn.textContent = '自动安装';
        btn.id = 'install-' + r.install_cmd;
        btn.onclick = function () {
          btn.disabled = true;
          btn.textContent = '正在安装...';
          window.ipc.postMessage(r.install_cmd);
        };
        row.appendChild(btn);
      }
      list.appendChild(row);
    });
    panel.style.display = 'block';
    if (allOk) {
      // 全部通过：面板保持显示，不再单独等待转圈，直接进入插件加载
      document.getElementById('spinner').classList.add('hidden');
      setStatus('环境检查通过，正在启动服务...');
    } else {
      document.getElementById('spinner').classList.add('hidden');
      document.getElementById('retry').classList.remove('hidden');
      document.getElementById('exit').classList.remove('hidden');
    }
  }
</script>
</body>
</html>"#
        .to_string()
}

/// 注入导航栏到 DSH 页面（Shadow DOM 隔离，防插件主题 CSS 穿透）。每次页面加载后调用。
pub fn inject_navbar_script() -> String {
    r#"(function(){
  if (document.getElementById('dsh-navbar-host')) return;
  var host = document.createElement('div');
  host.id = 'dsh-navbar-host';
  // 内联 !important 定位，防止插件主题 CSS 覆盖
  host.style.setProperty('position', 'fixed', 'important');
  host.style.setProperty('top', '0px', 'important');
  host.style.setProperty('left', '0px', 'important');
  host.style.setProperty('right', '0px', 'important');
  host.style.setProperty('bottom', 'auto', 'important');
  host.style.setProperty('height', '20px', 'important');
  host.style.setProperty('background', 'transparent', 'important');
  host.style.setProperty('z-index', '2147483647', 'important');
  host.style.setProperty('pointer-events', 'none', 'important');
  host.style.setProperty('transform', 'none', 'important');

  // Shadow DOM：内部样式与页面完全隔离，插件 CSS 无法穿透
  var root = host.attachShadow({ mode: 'open' });
  root.innerHTML = '<style>'
    + ':host{all:initial;display:block!important;width:100%!important;height:20px!important;font-family:"Segoe UI",system-ui,sans-serif!important;}'
    + '.bar{display:flex!important;flex-direction:row!important;align-items:center!important;justify-content:flex-start!important;height:20px!important;width:100%!important;padding:0 6px!important;gap:3px!important;box-sizing:border-box!important;}'
    + '.n-btn{width:18px;height:16px;max-width:18px;max-height:16px;min-width:18px;min-height:16px;border:none;border-radius:3px;background:transparent;cursor:pointer;display:flex;align-items:center;justify-content:center;position:relative;padding:0;margin:0;pointer-events:auto;box-shadow:none;outline:none;font-size:0;box-sizing:border-box;color:#888;}'
    + '.n-btn:hover{background:#333;}'
    + '.n-btn.active{background:#4d6bfe;}'
    + '.n-btn svg{width:12px;height:12px;max-width:12px;max-height:12px;min-width:12px;min-height:12px;fill:none;stroke:#888;stroke-width:2;stroke-linecap:round;stroke-linejoin:round;display:block;margin:0;padding:0;flex:none;box-sizing:content-box;}'
    + '.n-btn:hover svg{stroke:#fff;}'
    + '.n-btn.active svg{stroke:#fff;}'
    + '.n-tip{position:absolute;top:100%;left:50%;transform:translateX(-50%);margin-top:2px;padding:2px 6px;background:#333;color:#fff;font-size:10px;line-height:1.2;border-radius:3px;white-space:nowrap;pointer-events:none;opacity:0;transition:opacity .15s;z-index:999999;font-family:"Segoe UI",system-ui,sans-serif;}'
    + '.n-btn:hover .n-tip{opacity:1;}'
    + '.n-spacer{flex:1 1 auto;}'
    + '.n-dot{position:absolute;top:1px;right:1px;width:6px;height:6px;border-radius:50%;background:#ff4444;display:none;}'
    + '.n-dot.show{display:block;}'
    + '.n-prompt{position:fixed;bottom:60px;left:50%;transform:translateX(-50%);background:#2a2a2a;border:1px solid #555;border-radius:6px;padding:8px 16px;font-size:12px;color:#eee;display:none;z-index:2147483647;font-family:"Segoe UI",system-ui,sans-serif;white-space:nowrap;pointer-events:auto;cursor:pointer;}'
    + '.n-prompt.show{display:flex;align-items:center;gap:8px;}'
    + '.n-prompt:hover{background:#3a3a3a;}'
    + '</style>'
    + '<div class="bar">'
    + '<button class="n-btn">'
      + '<svg viewBox="0 0 24 24"><path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/></svg>'
      + '<span class="n-tip">设置</span>'
    + '</button>'
    + '<button class="n-btn" data-cmd="refresh">'
      + '<svg viewBox="0 0 24 24"><polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/></svg>'
      + '<span class="n-tip">刷新页面</span>'
    + '</button>'
    + '<button class="n-btn" data-cmd="restart">'
      + '<span class="n-dot" id="dsh-update-dot"></span>'
      + '<svg viewBox="0 0 24 24"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>'
      + '<span class="n-tip">重启服务</span>'
    + '</button>'
    + '<button class="n-btn" id="dsh-btn-exit" data-cmd="toggle-exit-mode">'
      + '<svg viewBox="0 0 24 24"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>'
      + '<span class="n-tip">关闭时退出</span>'
    + '</button>'
    + '<button class="n-btn" id="dsh-btn-tray" data-cmd="toggle-tray">'
      + '<svg viewBox="0 0 24 24"><rect x="4" y="2" width="16" height="20" rx="2"/><line x1="9" y1="6" x2="15" y2="6"/></svg>'
      + '<span class="n-tip">常驻任务栏</span>'
    + '</button>'
    + '<span class="n-spacer"></span>'
    + '</div>'
    + '<div class="n-prompt" id="dsh-update-prompt">已更新，<span style="color:#4d6bfe;text-decoration:underline;">点击重启</span></div>';

  // 更新红点提示函数
  window.showUpdateDot = function() {
    var dot = root.getElementById('dsh-update-dot');
    if (dot) dot.classList.add('show');
    var prompt = root.getElementById('dsh-update-prompt');
    if (prompt) prompt.classList.add('show');
  };

  // 更新提示点击重启
  root.getElementById('dsh-update-prompt').addEventListener('click', function() {
    if (window.ipc && window.ipc.postMessage) { window.ipc.postMessage('restart'); }
  });

  // 点击事件：Shadow DOM 内的按钮通过 data-cmd 通知宿主 IPC
  root.querySelector('.bar').addEventListener('click', function(e){
    var btn = e.target.closest ? e.target.closest('.n-btn') : null;
    if (btn && btn.dataset && btn.dataset.cmd) {
      var msg = btn.dataset.cmd;
      if (window.ipc && window.ipc.postMessage) { window.ipc.postMessage(msg); }
      else if (window.parent && window.parent.ipc && window.parent.ipc.postMessage) { window.parent.ipc.postMessage(msg); }
    }
  });

  // 挂到 html 根元素（比 body 更不容易被插件容器包裹/加 transform）
  (document.documentElement || document.body).appendChild(host);

  // 兜底自检：防止插件把导航栏挪走或样式覆盖
  var assert = function(){
    var h = document.getElementById('dsh-navbar-host');
    if (!h) { return; }
    if (h.parentNode !== document.documentElement && h.parentNode !== document.body) {
      (document.documentElement || document.body).appendChild(h);
    }
    h.style.setProperty('position', 'fixed', 'important');
    h.style.setProperty('top', '0px', 'important');
    h.style.setProperty('left', '0px', 'important');
    h.style.setProperty('right', '0px', 'important');
    h.style.setProperty('bottom', 'auto', 'important');
    h.style.setProperty('height', '20px', 'important');
    h.style.setProperty('z-index', '2147483647', 'important');
    h.style.setProperty('transform', 'none', 'important');
    try {
      var r = h.getBoundingClientRect();
      if (Math.abs(r.top) > 1 || Math.abs(r.bottom - window.innerHeight) < 30) {
        h.style.setProperty('top', '0px', 'important');
        h.style.setProperty('bottom', 'auto', 'important');
      }
    } catch(e) {}
  };
  var iv = setInterval(assert, 1000);
  window.setTimeout(function(){ clearInterval(iv); }, 180000);
  assert();
})();"#
        .to_string()
}

/// 更新导航栏按钮状态
pub fn nav_set_exit_mode(enabled: bool) -> String {
    let active_class = if enabled { " active" } else { "" };
    let tip = if enabled { "关闭时退出 ✓" } else { "关闭时隐藏" };
    format!(
        "(function(){{var h=document.getElementById('dsh-navbar-host');var r=h?h.shadowRoot:null;var b=r?r.getElementById('dsh-btn-exit'):null;if(b){{b.className='n-btn{ac}'}};
        var t=b?b.querySelector('.n-tip'):null;if(t)t.textContent='{tip}';}})();",
        ac = active_class,
        tip = tip
    )
}

pub fn nav_set_tray_mode(enabled: bool) -> String {
    let active_class = if enabled { " active" } else { "" };
    let tip = if enabled { "常驻任务栏 ✓" } else { "不驻任务栏" };
    format!(
        "(function(){{var h=document.getElementById('dsh-navbar-host');var r=h?h.shadowRoot:null;var b=r?r.getElementById('dsh-btn-tray'):null;if(b){{b.className='n-btn{ac}'}};
        var t=b?b.querySelector('.n-tip'):null;if(t)t.textContent='{tip}';}})();",
        ac = active_class,
        tip = tip
    )
}

pub fn ui_msg_js(message: &UiMsg) -> String {
    match message {
        UiMsg::Step(text) => format!("setStatus({text:?});"),
        UiMsg::EnvProgress(name) => format!("showEnvProgress({name:?});"),
        UiMsg::EnvCheck(results) => {
            let items: Vec<String> = results
                .iter()
                .map(|r| {
                    format!(
                        "{{ok:{ok},name:{name},version:{version},error:{error},install_hint:{hint},install_cmd:{cmd}}}",
                        ok = if r.ok { "true" } else { "false" },
                        name = js_str(r.name),
                        version = js_str(r.version.as_deref().unwrap_or("")),
                        error = js_str(&r.error),
                        hint = js_str(r.install_hint),
                        cmd = js_str(r.install_cmd),
                    )
                })
                .collect();
            format!("showEnvCheck([{}]);", items.join(","))
        }
        UiMsg::Fail(error) => format!("showFail({error:?});"),
        UiMsg::Done(_) => "showDone();".into(),
    }
}

/// 生成 JS 字符串字面量（转义反斜杠、引号、换行、控制字符）
fn js_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
