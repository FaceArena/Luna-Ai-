/* luna-viewport.js
   Подключить ПЕРЕД основным скриптом (или вставить в начало него).
   Делает: --vvh / --vvt по visualViewport, класс .keyboard-open,
   сохраняет позицию ленты при открытии/закрытии клавиатуры,
   не даёт Safari сдвигать страницу, держит клавиатуру открытой после отправки. */
(function () {
  var root = document.documentElement;
  var vv = window.visualViewport;
  var log = document.getElementById('log');
  var ta = document.querySelector('#shell > footer textarea');
  var send = document.querySelector('#shell > footer .send');
  var dist = 0;      // расстояние от низа ленты до текущего скролла
  var raf = 0;

  function measure() {
    if (log) dist = log.scrollHeight - log.scrollTop - log.clientHeight;
  }

  function apply() {
    raf = 0;
    var h = vv ? vv.height : window.innerHeight;
    var top = vv ? vv.offsetTop : 0;
    var kb = vv ? (window.innerHeight - vv.height) > 100 : false;

    root.style.setProperty('--vvh', Math.round(h) + 'px');
    root.style.setProperty('--vvt', Math.round(top) + 'px');
    root.classList.toggle('keyboard-open', kb);

    // сохранить позицию относительно низа (чат не "съезжает" и не обрезается под шапкой)
    if (log) log.scrollTop = log.scrollHeight - log.clientHeight - dist;
  }

  function schedule() {
    if (!raf) raf = requestAnimationFrame(apply);
  }

  function resetPageScroll() {
    window.scrollTo(0, 0);
    document.documentElement.scrollTop = 0;
    document.body.scrollTop = 0;
  }

  if (log) {
    log.addEventListener('scroll', measure, { passive: true });
  }

  if (vv) {
    vv.addEventListener('resize', function () { measure(); schedule(); });
    vv.addEventListener('scroll', function () { schedule(); resetPageScroll(); });
  }
  window.addEventListener('resize', schedule);
  window.addEventListener('orientationchange', function () { setTimeout(schedule, 300); });
  window.addEventListener('scroll', resetPageScroll, { passive: true });

  if (ta) {
    ta.addEventListener('focus', function () {
      [0, 50, 150, 300, 600].forEach(function (ms) {
        setTimeout(function () { resetPageScroll(); schedule(); }, ms);
      });
    });
    ta.addEventListener('blur', function () {
      [0, 100, 300].forEach(function (ms) {
        setTimeout(function () { resetPageScroll(); schedule(); }, ms);
      });
    });
  }

  // Клавиатура не закрывается после отправки
  if (send && ta) {
    send.addEventListener('pointerdown', function (e) {
      if (document.activeElement === ta) e.preventDefault();
    });
    // синхронно, внутри жеста пользователя: так iOS оставляет клавиатуру
    send.addEventListener('click', function () {
      ta.focus({ preventScroll: true });
    }, true);
  }

  // Тап по ленте закрывает клавиатуру
  if (log && ta) {
    var startY = 0;
    log.addEventListener('touchstart', function (e) { startY = e.touches[0].clientY; }, { passive: true });
    log.addEventListener('touchmove', function (e) {
      if (document.activeElement === ta && e.touches[0].clientY - startY > 40) ta.blur();
    }, { passive: true });
  }

  measure();
  apply();
})();
