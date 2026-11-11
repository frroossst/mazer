-- mazer.lua: Neovim support for mazer notes (.zr) through mazer-lsp.
--
-- Install:
--   1. Put the server on your PATH:
--        cargo install --path mazer-lsp        (from the mazer repository)
--   2. Make this file loadable as the Lua module `mazer`, e.g.
--        ln -s /path/to/mazer/mazer-lsp/nvim/mazer.lua ~/.config/nvim/lua/mazer.lua
--   3. In init.lua:
--        require('mazer')            -- or require('mazer').setup({ autocomplete = false })
--
-- What it does: registers the `zr` filetype for *.zr, starts `mazer-lsp` for
-- each .zr buffer (root = the file's directory), links the server's semantic
-- token types to highlight groups, turns on Neovim's built-in completion
-- popup (unless `autocomplete = false`, e.g. when you use nvim-cmp), shows
-- signature help after a space or `(` in insert mode (unless
-- `signature = false`), and adds :MazerPreview, which opens a live preview of
-- the note in your browser.
-- Needs Neovim 0.11 or newer. No plugins required.

local M = {}

local defaults = { cmd = { 'mazer-lsp' }, autocomplete = true, signature = true }

-- Semantic token types the server sends, and the groups they look like.
local links = {
  keyword = '@keyword', -- show, let, alias, macro, ref, eval, tag
  ['function'] = '@function', -- keyword calls, function application
  macro = '@constant.macro', -- alias and macro uses
  enumMember = '@character.special', -- display names: tau, alpha, ->
  variable = '@variable', -- let variables
  parameter = '@variable.parameter', -- macro parameters, other names
  number = '@number',
  string = '@string',
  comment = '@comment',
  operator = '@operator', -- the ' of 'x and the - of -x
  label = '@label', -- the names in (tag name) and (ref name)
}

function M.setup(opts)
  opts = vim.tbl_extend('force', defaults, opts or {})
  vim.filetype.add({ extension = { zr = 'zr' } })

  for token, group in pairs(links) do
    vim.api.nvim_set_hl(0, '@lsp.type.' .. token .. '.zr', { link = group, default = true })
  end
  vim.api.nvim_set_hl(0, '@lsp.typemod.function.defaultLibrary.zr', { link = '@function.builtin', default = true })

  local group = vim.api.nvim_create_augroup('mazer', { clear = true })
  vim.api.nvim_create_autocmd('FileType', {
    group = group,
    pattern = 'zr',
    callback = function(args)
      if vim.fn.executable(opts.cmd[1]) == 0 then
        vim.notify_once(opts.cmd[1] .. ' is not on PATH: cargo install --path mazer-lsp', vim.log.levels.WARN)
        return
      end
      vim.lsp.start({
        name = 'mazer',
        cmd = opts.cmd,
        root_dir = vim.fs.dirname(vim.api.nvim_buf_get_name(args.buf)),
      }, { bufnr = args.buf })
    end,
  })

  vim.api.nvim_create_autocmd('LspAttach', {
    group = group,
    callback = function(args)
      local client = vim.lsp.get_client_by_id(args.data.client_id)
      if not client or client.name ~= 'mazer' then
        return
      end
      if opts.autocomplete and vim.lsp.completion then
        vim.lsp.completion.enable(true, client.id, args.buf, { autotrigger = true })
      end
      if opts.signature then
        vim.api.nvim_clear_autocmds({ group = group, buffer = args.buf, event = 'TextChangedI' })
        vim.api.nvim_create_autocmd('TextChangedI', {
          group = group,
          buffer = args.buf,
          callback = function()
            local col = vim.api.nvim_win_get_cursor(0)[2]
            local typed = vim.api.nvim_get_current_line():sub(col, col)
            if typed == ' ' or typed == '(' then
              vim.lsp.buf.signature_help({ silent = true, focusable = false })
            end
          end,
        })
      end
      vim.api.nvim_buf_create_user_command(args.buf, 'MazerPreview', M.preview, { desc = 'Live preview of this note' })
    end,
  })
end

-- Asks the server for a live preview of the current note and opens its URL.
function M.preview()
  local buf = vim.api.nvim_get_current_buf()
  local client = vim.lsp.get_clients({ bufnr = buf, name = 'mazer' })[1]
  if not client then
    vim.notify('mazer-lsp is not attached to this buffer', vim.log.levels.ERROR)
    return
  end
  local params = { command = 'mazer.preview', arguments = { vim.uri_from_bufnr(buf) } }
  client:request('workspace/executeCommand', params, function(err, url)
    if err or type(url) ~= 'string' then
      vim.notify('mazer preview failed: ' .. (err and err.message or 'no URL'), vim.log.levels.ERROR)
      return
    end
    vim.ui.open(url)
  end, buf)
end

M.setup()

return M
