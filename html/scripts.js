function showCustomAlert(message) {
  var alertBox = document.getElementById('customAlert');
  var alertText = document.getElementById('customAlertText');
  alertText.innerHTML = message;
  alertBox.style.backgroundColor = '#fff'; // Default color
  alertBox.style.display = 'block';
}

function closeCustomAlert() {
  var alertBox = document.getElementById('customAlert');
  alertBox.style.display = 'none';
}

var socket = new WebSocket("ws://localhost:8080/ws");

var currentUserId = undefined;
var currentSensitiveCategory = 'alcohol';


const SENSITIVE_AISLES = {
    'alcohol': [27, 28, 62, 124, 134],
    'meat': [5, 15, 33, 34, 35, 49, 95, 96, 106, 122],
    'unhealthy': [37, 38, 45, 61, 77, 79, 106],
};

function isSensitive(itemId) {
    return SENSITIVE_AISLES[currentSensitiveCategory].includes(AISLES[itemId]);
}

socket.onmessage = function (event) {
    var messages = document.getElementById("messages");
    messages.innerHTML = event.data + "\n" + messages.innerHTML;

    var response = JSON.parse(event.data);

    if (response["response_type"] == "purchases") {

        currentUserId = response["payload"]["user_id"];

        var currentBasket = -1;
        var basketItems = "";
        for (index in response["payload"]["item_purchases"]) {
            var basket = response["payload"]["item_purchases"][index]["basket_id"];
            var itemId = response["payload"]["item_purchases"][index]["item_id"]
            var itemName = PRODUCTS[itemId];
            if (isSensitive(itemId)) {
                itemName = '<span style="font-weight:bold;">' + itemName + '</span>';
                itemName += ' <a href="javascript:deletePurchase(' + response["payload"]["user_id"] + ', ' +  itemId + ')">[unlearn]</a>';
            }
            basketItems += itemName + ", ";
            if (basket != currentBasket) {

                var day = response["payload"]["item_purchases"][index]["day_of_week"];
                var hour = response["payload"]["item_purchases"][index]["hour_of_day"];

                const weekDays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

                var time = weekDays[day] + ', ' + hour + ':00 - ';

                if (currentBasket != -1) {
                    basketItems += "</li><li>" + time;
                } else {
                    basketItems = "<ul><li>" + time + basketItems;
                }
                currentBasket = basket;
            }
        }
        basketItems += "</li></ul>";
        document.getElementById("baskets").innerHTML = basketItems;
    }

    if (response["response_type"] == "neighbors") {

        var vertices = [];
        var edges = [];

        vertices.push({ data: { id: currentUserId, name: currentUserId, weight: 20.0 }});

        for (index in response["payload"]["adjacent"]) {
            var neighborId = response["payload"]["adjacent"][index][0];
            var similarity = response["payload"]["adjacent"][index][1];
            vertices.push({ data: { id: neighborId, name: neighborId, weight: similarity * 100 }});
            edges.push({ data: { id: currentUserId + '-' + neighborId, source: currentUserId, target: neighborId }})
        }

        for (index in response["payload"]["incident"]) {
            var neighborId = response["payload"]["incident"][index][0];
            var similarity = response["payload"]["incident"][index][1];
            vertices.push({ data: { id: neighborId, name: neighborId, weight: similarity * 100 }});
            edges.push({ data: { id: neighborId + '-' + currentUserId, source: neighborId, target: currentUserId }})
        }

        cy.elements().remove();
        cy.json({ elements: { nodes: vertices, edges: edges }});

        var aisleDistribution = 'Top product categories in neighborhood -- ';
        for (index in response["payload"]["top_aisles"]) {
            var aisleId = response["payload"]["top_aisles"][index][0];
            var percentage = response["payload"]["top_aisles"][index][1];

            var aisleName = AISLE_NAMES[aisleId];

            // Hack...
            if (SENSITIVE_AISLES[currentSensitiveCategory].includes(aisleId)) {
                aisleName = '<b>' + aisleName + '</b>';
            }

            aisleName += ' <img src="images/aisles/' + aisleId + '.png" style="width: 30px; height: 30px;"/>'

            aisleDistribution +=  aisleName + ': ' + (percentage.toFixed(3) * 100).toFixed(1) + '%, ';
        }

        document.getElementById('neighbors').innerHTML = 'This user influences the recommendations of ' + response["payload"]["incident"].length +
        ' other users.' +
        '<br/><br/>' + aisleDistribution;

        var layout = cy.layout({
            name: 'breadthfirst',
            directed: true,
            padding: 2,
            spacingFactor: 3,
            depthSort: function(a, b) { return b.data('weight') - a.data('weight') }
        });
        layout.run();
    }

    if (response["response_type"] == "recommendations") {
        const html = response["payload"]
            .map(e => {
                var itemId = e[0];
                var itemName = PRODUCTS[itemId];
                if (isSensitive(itemId)) {
                    itemName = '<span style="color:red;">' + itemName + '</span>';
                }
                return itemName + " (" + e[1].toFixed(3) + ")"
            })
            .join(", ");
        document.getElementById("recommendations").innerHTML = html;
    }

    if (response["response_type"] == "embedding") {
        const html = response["payload"]["weights_per_item"]
            .map(e => {
                var itemId = e[0];
                var itemName = PRODUCTS[itemId];
                if (isSensitive(itemId)) {
                    itemName = '<span style="color:red;">' + itemName + '</span>';
                }
                return itemName + " (" + e[1].toFixed(3) + ")"
            })
            .join(", ");
        document.getElementById("embedding").innerHTML = html;
    }

    if (response["response_type"] == "deletion_impact") {
        //{"payload":{"num_inspected_neighbors":15944,"num_updated_neighbors":127,"user_id":93210},"response_type":"deletion_impact"}
        var userId = response["payload"]["user_id"]
        var itemId = response["payload"]["item_id"]
        var databaseUpdate = response["payload"]["database_update_duration"];
        var embeddingUpdate = response["payload"]["embedding_update_duration"];
        var indexUpdate = response["payload"]["topk_index_update_duration"];
        var query = response["payload"]["deletion_query"];

        const databaseDeletions = response["payload"]["basket_ids"]
            .map(basketId => '(' + basketId + ', ' + itemId + ')')
            .join(", ");

        const embeddingChanges = response["payload"]["embedding_difference"]
            .map(itemAndWeight => '(' + PRODUCTS[itemAndWeight[0]] + ', ' + itemAndWeight[1].toFixed(4) + ')')
            .join(", ");

        const recommendationChanges = response["payload"]["recommendation_difference"]
            .map(itemAndWeight => '(' + PRODUCTS[itemAndWeight[0]] + ', ' + itemAndWeight[1].toFixed(4) + ', ' + itemAndWeight[2] + ')')
            .join(", ");

        const topAisleChanges = response["payload"]["top_aisle_difference"]
            .map(itemAndWeight => '(' + PRODUCTS[itemAndWeight[0]] + ', ' + itemAndWeight[1].toFixed(4) + ', ' + itemAndWeight[2] + ')')
            .join(", ");

        const adjacentChanges = response["payload"]["adjacent_difference"]
            .map(itemAndWeight => '(' + itemAndWeight[0] + ', ' + itemAndWeight[1].toFixed(4) + ', ' + itemAndWeight[2] + ')')
            .join(", ");

        const incidentChanges = response["payload"]["incident_difference"]
            .map(itemAndWeight => '(' + itemAndWeight[0] + ', ' + itemAndWeight[1].toFixed(4) + ', ' + itemAndWeight[2] + ')')
            .join(", ");

        const update = `
          <h2>What changed in the database?</h2>
          <p>
            Deleted purchases for user ${response["payload"]["user_id"]} in ${databaseUpdate} ms with the following query:

            <pre>
                ${query}
            </pre>

            Deleted tuples: ${databaseDeletions}
          </p>


          <h2>What changed in the materialised recommendation model?</h2>
            <h4>Sparse user embedding</h4>
            <p>
                <ul>
                    <li>Update of the user embedding took ${embeddingUpdate} ms.</li>
                    <li>Embedding changes: ${embeddingChanges}</li>
                </ul>
            </p>
            <h4>Top-k neighborhood graph</h4>
            <p>
                <ul>
                    <li>Update of the top-k index took ${indexUpdate} ms</li>
                    <li>Maintenance of the top-k index involved inspection of the entries for ${response["payload"]["num_inspected_neighbors"]} users and
                     updates for the entries of ${response["payload"]["num_updated_neighbors"]} users.</li>
                </ul>
            </p>


          <h2>What changed in the recommendations for the user?</h2>
            <p>
                <ul>
                    <li>Recommendation changes: ${recommendationChanges}</li>
                </ul>
            </p>

          <h2>What changed in how this user influences the recommendations of other users?</h2>
            <p>
                <ul>
                    <li>TODO COUNT CHANGES</li>
                    <li>Top aisle changes: ${topAisleChanges}</li>
                </ul>
            </p>
        `;
        showCustomAlert(update);
    }
}

function userFocus(userId) {
    var request = { "UserFocus": { "user_id": userId }};
    socket.send(JSON.stringify(request));
}

function deletePurchase(userId, itemId) {
    var request = { "PurchaseDeletion": { "user_id": userId, "item_id": itemId }};
    socket.send(JSON.stringify(request));
}

function openTab(tabName) {
    const tabs = document.getElementsByClassName("tab");
    for (let i = 0; i < tabs.length; i++) {
        tabs[i].classList.remove("active-tab");
    }

    const tabButtons = document.getElementsByClassName("tab-button");
    for (let i = 0; i < tabButtons.length; i++) {
        tabButtons[i].classList.remove("active-tab-button");
    }

    // Yes, this is an ugly hack
    currentSensitiveCategory = tabName;

    const selectedTab = document.getElementById(tabName);
    selectedTab.classList.add("active-tab");
    const selectedTabButton = document.getElementById(tabName + '-button');
    selectedTabButton.classList.add("active-tab-button");
}