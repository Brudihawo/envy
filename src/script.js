function matches_tags(tags, filter) {
  if (filter == 'READ' || filter == 'UNREAD' || filter == 'READING') {
    return tags.map((tag) => tag.toUpperCase()).includes(filter);
  }

  for (let i = 0; i < tags.length; i++) {
    if (tags[i].toUpperCase().includes(filter)) {
      return true;
    }
  }
  return false;
}

function filter_list(list_id, query_id) {
  var input = document.getElementById(query_id);
  var filter = input.value.toUpperCase();
  var ul = document.getElementById(list_id);
  li = ul.getElementsByTagName('li');

  var a;
  for (let i = 0; i < li.length; ++i) {
    a = li[i].getElementsByTagName('a')[0];
    var tags = li[i].getAttribute("tags").split(", ");
    var title = li[i].getAttribute("title").toUpperCase();
    var authors = li[i].getAttribute("authors").toUpperCase();

    if (filter == "" || matches_tags(tags, filter) || title.includes(filter) || authors.includes(filter)) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }
}

function open_search() {
  let search_parent = document.getElementById("search_parent");
  search_parent.style.display = "block";
  document.getElementById('search_input').focus();
}

function close_search() {
  let search_parent = document.getElementById("search_parent");
  search_parent.style.display = "none";
}


function process_up(e) {
  if (e.key == "k" && e.ctrlKey) {
    e.preventDefault();

    console.log("Pressed Ctrl-K")
  }
}

function process_down(e) {
  if (e.key === "k" && e.ctrlKey) {
    e.preventDefault();
    let search_parent = document.getElementById("search_parent");
    if (search_parent.style.display === "none") {
      open_search();
    } else {
      close_search();
    }
    return;
  }

  if (e.key === "Escape") {
    close_search();
    e.preventDefault();
    return
  }

}

async function update_search() {
  let search_input = document.getElementById('search_input').value;
  let url = window.location.origin + "/api?any=" + search_input;
  console.log(url);
  let response = await fetch(url).then((res) => res.text());
  let res_div = document.getElementById("search_res_div");
  res_div.innerHTML = response;
}

function get_cookie(cname) {
  let name = cname + "=";
  let decoded_cookie = decodeURIComponent(document.cookie);
  let ca = decoded_cookie.split(";");
  for (let i = 0; i < ca.length; ++i) {
    let c = ca[i];
    while (c.charAt(0) == ' ') {
      c = c.substring(1);
    }
    if (c.indexOf(name) == 0) {
      return c.substring(name.length, c.length);
    }
  }
  return "";
}

function update_tab_display() {
  console.log("updating tabs")
  let parent = get_cookie("current_tab");
  if (parent === "") {
    parent = document.getElementsByClassName('tab-content')[0].getAttribute('parent');
    console.log("no tab selected, using '" + parent + "'");
  } else {
    console.log("Tab selected, using '" + parent + "'");
  }

  let tab_content_id = parent + '-content';
  let tab_title_id = parent + '-tab';

  for (element of document.getElementsByClassName('tab-content')) {
    if (element.id === tab_content_id) {
      element.style.display = "block";
    } else {
      element.style.display = "none";
    }
  }

  for (element of document.getElementsByClassName('tab')) {
    if (element.id === tab_title_id) {
      element.style.color = "#fefefe";
    } else {
      element.style.color = "#646464";
    }
  }
}

function update_radios() {
  var radios = document.getElementById('tabbed-radios').children;
  for (let i = 0; i < radios.length; ++i) {
    let radio = radios[i];
    if (radio.checked) {
      let parent = radio.getAttribute('parent');
      document.cookie = "current_tab=" + parent +";SameSite=Strict;";
      break;
    }
  }

  update_tab_display();
}
