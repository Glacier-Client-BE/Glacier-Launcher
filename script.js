document.getElementById('launch-game').addEventListener('click', function() {
  const userAgent = navigator.userAgent || navigator.vendor;
  if (/android/i.test(userAgent)) {
    window.location.href = 'intent://#Intent;scheme=minecraft;package=com.mojang.minecraftpe;end;';
  } else {
    window.location.href = 'minecraft://';
  }
});
