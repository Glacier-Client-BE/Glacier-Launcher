// Real port of the desktop app's own wwwroot/js/interop.js panel-tab-bar
// cycler — already plain browser JS, portable as-is. Bedrock/Java panel
// footers carry 4-11 tabs; rather than a partial-scroll strip, the bar shows
// one page of up to 4 tabs at a time with sticky prev/next arrows to
// paginate, exactly like desktop.
(function () {
    const PAGE_SIZE = 4;

    function tabsOf(bar) {
        return Array.from(bar.querySelectorAll(':scope > .panel-tab'));
    }

    function refreshCyclerButtons(bar) {
        const prev = bar.querySelector(':scope > .panel-tabs-arrow.prev');
        const next = bar.querySelector(':scope > .panel-tabs-arrow.next');
        if (!prev || !next) return;

        const tabs = tabsOf(bar);
        const totalPages = Math.max(1, Math.ceil(tabs.length / PAGE_SIZE));
        let page = parseInt(bar.dataset.page || '0', 10);

        const activeIdx = tabs.findIndex(t => t.classList.contains('active'));
        if (activeIdx >= 0) {
            const activePage = Math.floor(activeIdx / PAGE_SIZE);
            if (Math.floor(activeIdx / PAGE_SIZE) !== page && bar.dataset.userPaged !== String(page)) {
                page = activePage;
            }
        }
        page = Math.min(Math.max(page, 0), totalPages - 1);
        bar.dataset.page = String(page);

        tabs.forEach((tab, i) => {
            tab.style.display = Math.floor(i / PAGE_SIZE) === page ? '' : 'none';
        });

        const overflowing = totalPages > 1;
        prev.style.display = overflowing && page > 0 ? 'flex' : 'none';
        next.style.display = overflowing && page < totalPages - 1 ? 'flex' : 'none';
    }

    function ensureTabCyclers() {
        document.querySelectorAll('.panel-tabs:not(.cycler-ready)').forEach(bar => {
            bar.classList.add('cycler-ready');
            bar.dataset.page = '0';

            const prev = document.createElement('button');
            prev.type = 'button';
            prev.className = 'panel-tabs-arrow prev';
            prev.title = 'Previous tabs';
            prev.innerHTML = '<i class="fa-solid fa-chevron-left"></i>';
            prev.addEventListener('click', e => {
                e.stopPropagation();
                const p = Math.max(0, parseInt(bar.dataset.page || '0', 10) - 1);
                bar.dataset.page = String(p);
                bar.dataset.userPaged = String(p);
                refreshCyclerButtons(bar);
            });

            const next = document.createElement('button');
            next.type = 'button';
            next.className = 'panel-tabs-arrow next';
            next.title = 'Next tabs';
            next.innerHTML = '<i class="fa-solid fa-chevron-right"></i>';
            next.addEventListener('click', e => {
                e.stopPropagation();
                const totalPages = Math.max(1, Math.ceil(tabsOf(bar).length / PAGE_SIZE));
                const p = Math.min(totalPages - 1, parseInt(bar.dataset.page || '0', 10) + 1);
                bar.dataset.page = String(p);
                bar.dataset.userPaged = String(p);
                refreshCyclerButtons(bar);
            });

            bar.insertBefore(prev, bar.firstChild);
            bar.appendChild(next);
            refreshCyclerButtons(bar);
        });

        document.querySelectorAll('.panel-tabs.cycler-ready').forEach(refreshCyclerButtons);
    }

    const tabsObserver = new MutationObserver(() => ensureTabCyclers());
    tabsObserver.observe(document.body, { childList: true, subtree: true });
    window.addEventListener('resize', ensureTabCyclers);
    ensureTabCyclers();
})();
