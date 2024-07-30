function matches(tags, filter) {
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

function filter(list_id, query_id) {
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

    if (filter == "" || matches(tags, filter) || title.includes(filter) || authors.includes(filter)) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }
}

function filter_tags(list_id, query_id) {
  var input = document.getElementById(query_id);
  var filter = input.value.toUpperCase();
  var ul = document.getElementById(list_id);
  li = ul.getElementsByTagName('li');

  for (let i = 0; i < li.length; ++i) {
    tag = li[i].textContent.toUpperCase();

    if (filter == "" || tag.includes(filter)) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }
}
