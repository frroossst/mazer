-- test_typing.lua: types into fresh .zr buffers key by key, the way a person
-- does, with mazer.lua's built-in LSP completion switched on, and checks that
-- each buffer holds exactly what was typed. Then it picks completion items on
-- purpose (<C-n> then <C-y>) and checks what they insert.
--
-- Every key goes through Neovim's real input loop in Insert mode
-- (nvim_input from a timer), and between keys the script waits for the
-- completion autotrigger to fire and for every LSP response to arrive, so the
-- popup menu is really open while typing goes on.
--
-- Run it from anywhere, with a built mazer-lsp on PATH:
--
--   cargo build -p mazer-lsp
--   PATH="$CARGO_TARGET_DIR/debug:$PATH" \
--     nvim --clean --headless -c "luafile mazer-lsp/nvim/test_typing.lua"
--
-- It prints one line per case and exits with status 0 when every case passes,
-- 1 otherwise. `cargo test -p mazer-lsp --test nvim_typing` runs it when nvim
-- is installed.

vim.o.showmode = false
vim.opt.shortmess:append('c')
local here = vim.fn.fnamemodify(debug.getinfo(1, 'S').source:sub(2), ':p:h')
local mazer = dofile(here .. '/mazer.lua')
assert(mazer, 'mazer.lua did not load')

local dir = vim.fn.tempname()
vim.fn.mkdir(dir, 'p')

local out = {}
local failures = 0
local function say(line)
  table.insert(out, line)
  io.stdout:write(line .. '\n')
end

local co

local function resume()
  local ok, err = coroutine.resume(co)
  if not ok then
    say('ERROR ' .. tostring(err))
    vim.cmd('cquit 1')
  end
end

local function sleep(ms)
  vim.defer_fn(resume, ms)
  coroutine.yield()
end

local function client()
  return vim.lsp.get_clients({ bufnr = 0, name = 'mazer' })[1]
end

-- Waits until the autotrigger timer has fired and no request is pending.
local menus = 0
local function settle()
  sleep(60)
  local c = client()
  local waited = 0
  while c and next(c.requests) ~= nil and waited < 3000 do
    sleep(10)
    waited = waited + 10
  end
  sleep(40)
  if vim.fn.pumvisible() == 1 then
    menus = menus + 1
  end
end

-- `keys` is literal text, except `<CR>`, `<C-n>`, `<C-y>`, `<C-x>`, `<C-o>`
-- and `<Esc>`, which are those keys.
local function press(keys)
  local i = 1
  while i <= #keys do
    local special = keys:match('^<[%w%-]+>', i)
    local key
    if special then
      key = special
      i = i + #special
    else
      local ch = vim.fn.strcharpart(keys:sub(i), 0, 1)
      key = ch == '<' and '<lt>' or ch
      i = i + #ch
    end
    vim.api.nvim_input(key)
    settle()
  end
end

local n = 0
local function fresh_buffer()
  vim.api.nvim_input('<Esc>')
  sleep(50)
  n = n + 1
  vim.cmd('silent! %bwipeout!')
  vim.cmd.edit(dir .. '/case' .. n .. '.zr')
  local waited = 0
  while not (client() and vim.lsp.buf_is_attached(0, client().id)) do
    sleep(20)
    waited = waited + 20
    if waited > 10000 then
      error('mazer-lsp did not attach (is it on PATH?)')
    end
  end
  vim.api.nvim_input('i')
  settle()
end

local function show(lines)
  return vim.inspect(table.concat(lines, '\n'))
end

local function check(name, typed, want)
  local got = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local ok = vim.deep_equal(got, want)
  if not ok then
    failures = failures + 1
  end
  say(string.format('%s %-28s typed %s -> buffer %s', ok and 'ok  ' or 'FAIL', name, vim.inspect(typed), show(got)))
  if not ok then
    say('     wanted ' .. show(want))
  end
end

-- Types `text` and expects the buffer to hold exactly it.
local function typing(name, text)
  fresh_buffer()
  menus = 0
  press(text)
  local want = vim.split(text:gsub('<CR>', '\n'), '\n', { plain = true })
  check(name, text, want)
  say(string.format('     (the completion menu was open after %d of the keys)', menus))
end

local function main()
  local c = vim.o.completeopt
  -- Plain typing, with the menu popping up after `(`, `:` and `'`.
  typing('keyword call', '(show (frac a b))')
  say('     completeopt in a .zr buffer: ' .. vim.bo.completeopt .. ' (global: ' .. c .. ')')
  typing('inline show in prose', 'Text (show y) end')
  typing('let then show', '(let x 2)<CR>(show (+ x 1))')
  typing('emoji and ref', ':sob: and (ref eq)')
  typing('integral', '(show (integral (pow x 2) x 0 1))')
  typing('prose', "Well, (as noted: see below) it's fine, then: done (really).")
  typing('table', '(show (table (header a b) (row 1 2)))')

  -- Enter while the menu is open and nothing is selected: a newline only.
  fresh_buffer()
  press('Text (')
  local open = vim.fn.pumvisible() == 1
  say('     menu open before <CR>: ' .. tostring(open))
  if not open then
    failures = failures + 1
    say('FAIL the menu should be open after "Text ("')
  end
  press('<CR>more')
  check('enter with the menu open', 'Text (<CR>more', { 'Text (', 'more' })

  -- Choosing on purpose inserts that item.
  fresh_buffer()
  press('(show (fra')
  local open_head = vim.fn.pumvisible() == 1
  press('<C-n>')
  press('<C-y>')
  check('choose a keyword at a head', '(show (fra<C-n><C-y>', { '(show (frac num den' })
  say('     menu open before <C-n>: ' .. tostring(open_head))

  fresh_buffer()
  press('(show (+ alph')
  press('<C-x><C-o>')
  local open_arg = vim.fn.pumvisible() == 1
  press('<C-n>')
  press('<C-y>')
  check('choose a display name', '(show (+ alph<C-x><C-o><C-n><C-y>', { '(show (+ alpha' })
  say('     menu open before <C-n>: ' .. tostring(open_arg))

  fresh_buffer()
  press('So sad :so')
  press('<C-n>')
  press('<C-y>')
  check('choose an emoji', 'So sad :so<C-n><C-y>', { 'So sad :sob:' })

  vim.api.nvim_input('<Esc>')
  sleep(50)
  say(failures == 0 and 'all cases passed' or (failures .. ' case(s) failed'))
  vim.cmd(failures == 0 and 'qall!' or 'cquit 1')
end

co = coroutine.create(main)
vim.defer_fn(resume, 0)
