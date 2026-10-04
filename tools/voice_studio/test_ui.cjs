const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { JSDOM } = require('./runtime/ui-tests/node_modules/jsdom');
const calls = [];
let preview = '';
const state = {busy:false, phase:'Готов', error:'', job:'', settings:{minutes:240, synthetic:true, reference:''},
  rows:[], logs:[], installed:true, weights:true, server:false, free_gb:40, home:'C:/Studio',
  training:{}, test_run:'C:/Studio/runs/completed', test_info:''};
const dom = new JSDOM(fs.readFileSync(path.join(__dirname,'index.html'),'utf8'), {
  runScripts:'dangerously', url:'http://localhost/', beforeParse(window) {
    window.pywebview = {api:{get_state:async()=>state, get_preview:async()=>preview,
      action:async(name,data)=>{
        calls.push({name,data});
        if(name==='test_voice') {
          state.busy=true; state.job='Испытание голоса';
          setTimeout(()=>{
            state.busy=false; state.test_info=data.variant==='base'?'Исходный голос':'После обучения';
            preview='data:audio/wav;base64,UklGRg=='+calls.length;
          },40);
        }
        return {ok:true};
      }}};
  }
});
global.window=dom.window; global.document=dom.window.document;
const {screen,within,waitFor} = require('./runtime/ui-tests/node_modules/@testing-library/dom');
const userEvent = require('./runtime/ui-tests/node_modules/@testing-library/user-event').default;
(async()=>{
  const user = userEvent.setup({document:dom.window.document});
  dom.window.dispatchEvent(new dom.window.Event('pywebviewready'));
  const audition = within(screen.getByRole('region',{name:/испытание голоса/i}));
  await user.clear(audition.getByLabelText(/текст для испытания/i));
  await user.type(audition.getByLabelText(/текст для испытания/i),'Проверяем новый голос.');
  await user.click(audition.getByRole('button',{name:/испытать голос/i}));
  await waitFor(()=>assert.equal(calls.length,1));
  assert.deepEqual(JSON.parse(JSON.stringify(calls[0])),{name:'test_voice',data:{text:'Проверяем новый голос.',variant:'trained'}});
  await waitFor(()=>assert.equal(audition.getByRole('button',{name:/подготавливаю голос/i}).disabled,true));
  await waitFor(()=>assert.equal(audition.getByLabelText(/прослушать испытание голоса/i).parentElement.hidden,false),{timeout:3500});
  await user.click(audition.getByLabelText(/исходный голос/i));
  await user.click(audition.getByRole('button',{name:/испытать голос/i}));
  await waitFor(()=>assert.equal(calls.length,2));
  assert.equal(calls[1].data.variant,'base');
  state.training={phase:'interrupted',elapsed:74,remaining:14326,limit:14400,step:46};
  await user.click(screen.getByRole('button',{name:/^◷ Обучение$/i}));
  await screen.findByText(/обучение остановлено/i,{}, {timeout:3500});
  assert.equal(screen.queryByText(/до завершения/i),null);
  console.log('UI_OK: one click sends text and voice selection; loading blocks repeats; player appears.');
})().catch(e=>{console.error(e);process.exitCode=1}).finally(()=>dom.window.close());
