var nodes = document.querySelectorAll('.taskItem-title');
var list = [].slice.call(nodes);
var innertexts = list.map(function (e) { return e.innerText; }).join("\n");
var dummy = document.createElement("textarea");
document.body.appendChild(dummy);
dummy.value = innertexts;
dummy.select();

console.log(innertexts);